#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uuid::Uuid;
use walkdir::WalkDir;

const DRAFT_VERSION: u64 = 360000;
const FALLBACK_DURATION_US: i64 = 3_000_000;
const CANVAS_W: u32 = 1080;
const CANVAS_H: u32 = 1920;
const FPS: f64 = 30.0;
const TIME_BASE_DEN: i64 = 3000;

// 核心功能：支持场景分块识别的自然排序算法
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Chunk {
    Num(u64),
    Text(String),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Asset {
    name: String,
    path: String,
    kind: String,
    timestamp_ms: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct MatchPair {
    pair_id: usize,
    video: Option<Asset>,
    audio: Option<Asset>,
    status: String,
}

fn gen_id() -> String {
    Uuid::new_v4().to_string().to_uppercase()
}

fn extension_lower(path: &Path) -> String {
    path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase()
}

fn system_time_to_ms(st: SystemTime) -> u64 {
    st.duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO).as_millis() as u64
}

fn get_media_duration(path: &Path) -> i64 {
    let ext = extension_lower(path);
    match ext.as_str() {
        "mp4" | "mov" => {
            if let Ok(file) = std::fs::File::open(path) {
                if let Ok(meta) = file.metadata() {
                    if let Ok(mp4) = mp4::Mp4Reader::read_header(file, meta.len()) {
                        return (mp4.duration().as_secs_f64() * 1_000_000.0) as i64;
                    }
                }
            }
        }
        "wav" => {
            if let Ok(reader) = hound::WavReader::open(path) {
                let spec = reader.spec();
                if spec.sample_rate > 0 {
                    return ((reader.duration() as f64 / spec.sample_rate as f64) * 1_000_000.0) as i64;
                }
            }
        }
        "mp3" | "m4a" => {
            if let Ok(dur) = mp3_duration::from_path(path) {
                return (dur.as_secs_f64() * 1_000_000.0) as i64;
            }
        }
        _ => {}
    }
    FALLBACK_DURATION_US
}

// 提取器：多维分块解析，并排除扩展名背刺
fn extract_sort_key(name: &str, timestamp: u64) -> (Vec<Chunk>, u64) {
    let stem = Path::new(name).file_stem().and_then(|s| s.to_str()).unwrap_or(name);
    let mut clean_name = stem.to_lowercase();
    let media_stopwords = ["音频", "视频", "配音", "原声", "音效", "素材", "镜头", "画面", "场景"];
    for word in media_stopwords.iter() {
        clean_name = clean_name.replace(word, "");
    }
    clean_name = clean_name.replace(" ", "").replace("_", "");

    let mut chunks = Vec::new();
    let mut current_text = String::new();
    let mut current_num = String::new();

    for c in clean_name.chars() {
        if c.is_ascii_digit() {
            if !current_text.is_empty() {
                chunks.push(Chunk::Text(current_text.clone()));
                current_text.clear();
            }
            current_num.push(c);
        } else {
            if !current_num.is_empty() {
                if let Ok(n) = current_num.parse::<u64>() {
                    chunks.push(Chunk::Num(n));
                }
                current_num.clear();
            }
            current_text.push(c);
        }
    }
    if !current_text.is_empty() { chunks.push(Chunk::Text(current_text)); }
    if !current_num.is_empty() {
        if let Ok(n) = current_num.parse::<u64>() {
            chunks.push(Chunk::Num(n));
        }
    }

    (chunks, timestamp)
}

fn xml_escape(value: &str) -> String {
    value.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&apos;")
}

fn file_to_uri(path: &str) -> String {
    let p = path.replace("\\", "/");
    let mut encoded = String::new();
    for b in p.as_bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' | b':' => { encoded.push(*b as char); }
            _ => { encoded.push_str(&format!("%{:02X}", b)); }
        }
    }
    if encoded.starts_with('/') { format!("file://localhost{}", encoded) } else { format!("file://localhost/{}", encoded) }
}

fn us_to_frames(us: i64) -> i64 {
    if us <= 0 { 0 } else { ((us as f64 / 1_000_000.0) * FPS).round() as i64 }
}

fn frame_to_sec(frames: i64) -> String {
    format!("{}/{}s", frames * 100, TIME_BASE_DEN)
}

#[tauri::command]
fn smart_sniff_folder(path: String, multi_cam_sort: bool) -> Result<Vec<MatchPair>, String> {
    let mut visuals: Vec<Asset> = Vec::new();
    let mut audios: Vec<Asset> = Vec::new();

    for entry in WalkDir::new(&path).into_iter().filter_map(|e| e.ok()) {
        let path_buf = entry.path();
        if !path_buf.is_file() { continue; }
        let name = match path_buf.file_name().and_then(|n| n.to_str()) {
            Some(s) => s.to_string(), None => continue,
        };
        let name_lower = name.to_lowercase();
        if name_lower.starts_with('.') || name_lower.contains("draft_") { continue; }

        let metadata = fs::metadata(&path_buf).map_err(|e| format!("读取失败: {}", e))?;
        let timestamp_ms = system_time_to_ms(metadata.modified().unwrap_or(SystemTime::now()));
        let mut asset = Asset { name, path: path_buf.to_string_lossy().into_owned(), kind: String::new(), timestamp_ms };

        if name_lower.ends_with(".mp4") || name_lower.ends_with(".mov") {
            asset.kind = "video".into(); visuals.push(asset);
        } else if ["jpg", "jpeg", "png"].iter().any(|s| name_lower.ends_with(s)) {
            asset.kind = "image".into(); visuals.push(asset);
        } else if ["wav", "mp3", "m4a"].iter().any(|s| name_lower.ends_with(s)) {
            asset.kind = "audio".into(); audios.push(asset);
        }
    }

    // 核心修改：根据前端传来的多机位开关决定排序逻辑
    if multi_cam_sort {
        // 多机位模式：直接按物理拍摄时间戳排序
        visuals.sort_by(|a, b| a.timestamp_ms.cmp(&b.timestamp_ms));
        audios.sort_by(|a, b| a.timestamp_ms.cmp(&b.timestamp_ms));
    } else {
        // 智能模式（默认）：按文件名自然分块排序
        visuals.sort_by(|a, b| extract_sort_key(&a.name, a.timestamp_ms).cmp(&extract_sort_key(&b.name, b.timestamp_ms)));
        audios.sort_by(|a, b| extract_sort_key(&a.name, a.timestamp_ms).cmp(&extract_sort_key(&b.name, b.timestamp_ms)));
    }

    let max_len = std::cmp::max(visuals.len(), audios.len());
    let mut pairs = Vec::with_capacity(max_len);

    for i in 0..max_len {
        let video = visuals.get(i).cloned();
        let audio = audios.get(i).cloned();
        let status = match (&video, &audio) {
            (Some(_), Some(_)) => "perfect",
            (None, Some(_)) => "missing_video",
            (Some(_), None) => "missing_audio",
            (None, None) => "empty",
        };
        pairs.push(MatchPair { pair_id: i + 1, video, audio, status: status.to_string() });
    }
    Ok(pairs)
}

fn get_jianying_draft_dir() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("获取目录失败")?;
    #[cfg(target_os = "macos")]
    let base_path = home.join("Movies/JianyingPro/User Data/Projects/com.lveditor.draft");
    #[cfg(target_os = "windows")]
    let base_path = home.join("AppData/Local/JianyingPro/User Data/Projects/com.lveditor.draft");
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let base_path = home.join("Movies/JianyingPro/User Data/Projects/com.lveditor.draft");

    if base_path.exists() { Ok(base_path) } else { Err(format!("未找到剪映目录：{}", base_path.display())) }
}

fn link_media(src: &Path, dst: &Path) -> Result<(), String> {
    if let Some(parent) = dst.parent() { fs::create_dir_all(parent).map_err(|e| e.to_string())?; }
    if dst.exists() { fs::remove_file(dst).map_err(|e| e.to_string())?; }
    if fs::hard_link(src, dst).is_err() { fs::copy(src, dst).map_err(|e| e.to_string())?; }
    Ok(())
}

fn build_video_material(id: &str, path: &Path, name: &str, duration: i64, is_photo: bool) -> Value {
    json!({
        "id": id, "material_name": name, "media_path": "",
        "path": path.to_string_lossy().replace("\\", "/"),
        "type": if is_photo { "photo" } else { "video" },
        "width": CANVAS_W, "height": CANVAS_H, "duration": duration
    })
}

fn build_audio_material(id: &str, path: &Path, name: &str, duration: i64) -> Value {
    json!({
        "app_id": 0, "category_id": "", "category_name": "local", "check_flag": 3,
        "copyright_limit_type": "none", "duration": duration, "effect_id": "",
        "formula_id": "", "id": id, "local_material_id": id, "music_id": id,
        "name": name, "path": path.to_string_lossy().replace("\\", "/"),
        "source_platform": 0, "type": "extract_music", "wave_points": []
    })
}

fn build_meta_json(draft_path: &Path, folder_name: &str, audio_meta: Vec<Value>, video_meta: Vec<Value>) -> Value {
    let draft_path_string = draft_path.to_string_lossy().replace("\\", "/");
    json!({
        "draft_fold_path": draft_path_string, "draft_id": gen_id(), "draft_name": folder_name,
        "draft_materials": [
            { "type": 0, "value": [] }, { "type": 1, "value": video_meta },
            { "type": 2, "value": [] }, { "type": 3, "value": [] },
            { "type": 6, "value": [] }, { "type": 7, "value": [] },
            { "type": 8, "value": audio_meta }
        ],
        "draft_materials_copied_info": [], "draft_is_ai_shorts": false,
        "draft_is_ai_packaging_used": false, "draft_is_ai_translate": false,
        "draft_is_article_video_draft": false, "draft_is_from_deeplink": "false",
        "draft_is_invisible": false, "draft_new_version": "",
        "draft_root_path": draft_path_string, "draft_segment_extra_info": [],
        "draft_type": "", "tm_draft_cloud_completed": "",
        "tm_draft_cloud_modified": 0, "tm_draft_removed": 0, "tm_duration": 0
    })
}

#[tauri::command]
fn generate_draft(pairs: Vec<MatchPair>, _workspace: String, software: String, _multi_cam_sort: bool) -> Result<String, String> {
    
    // ========================================================
    // 工业级修复：达芬奇 FCPXML "扁平主轴"架构
    // ========================================================
    if software != "jianying" {
        let desktop = dirs::desktop_dir().ok_or("无法获取桌面路径")?;
        let file_name = format!("AI自动对齐序列_{}.fcpxml", software.to_uppercase());
        let export_path = desktop.join(file_name);

        let mut xml_resources = String::new();
        xml_resources.push_str(
            r#"        <format id="r1" name="FFVideoFormat1080p30" frameDuration="100/3000s" width="1080" height="1920" colorSpace="1-1-1 (Rec. 709)"/>
"#,
        );

        let mut xml_anchors = String::new();
        let mut total_frames: i64 = 0;

        for (i, pair) in pairs.iter().enumerate() {
            let v_dur = pair.video.as_ref().map_or(0, |v| get_media_duration(Path::new(&v.path)));
            let a_dur = pair.audio.as_ref().map_or(0, |a| get_media_duration(Path::new(&a.path)));
            let block_dur = if a_dur > 0 { a_dur } else { v_dur };
            if block_dur <= 0 { continue; }

            let block_frames = us_to_frames(block_dur);
            let offset_str = frame_to_sec(total_frames);

            if let Some(ref a) = pair.audio {
                let a_frames = us_to_frames(a_dur).min(block_frames);
                xml_resources.push_str(&format!(
                    r#"        <asset id="a_{i}" name="{name}" src="{uri}" start="0s" duration="{duration}" hasAudio="1" audioSources="1" audioChannels="2" audioRate="48k"/>
"#,
                    i = i, name = xml_escape(&a.name), uri = file_to_uri(&a.path), duration = frame_to_sec(a_frames)
                ));
                // 平行挂载！
                xml_anchors.push_str(&format!(
                    r#"                <asset-clip name="{name}" ref="a_{i}" offset="{offset}" start="0s" duration="{duration}" lane="-1" audioRole="dialogue"/>
"#,
                    name = xml_escape(&a.name), i = i, offset = offset_str, duration = frame_to_sec(a_frames)
                ));
            }

            if let Some(ref v) = pair.video {
                let v_frames = us_to_frames(v_dur);
                let is_photo = v.kind == "image";
                
                if is_photo {
                    xml_resources.push_str(&format!(
                        r#"        <asset id="v_{i}" name="{name}" src="{uri}" hasVideo="1" format="r1" type="still"/>
"#,
                        i = i, name = xml_escape(&v.name), uri = file_to_uri(&v.path)
                    ));
                    xml_anchors.push_str(&format!(
                        r#"                <asset-clip name="{name}" ref="v_{i}" offset="{offset}" start="0s" duration="{duration}" lane="1" videoRole="video"/>
"#,
                        name = xml_escape(&v.name), i = i, offset = offset_str, duration = frame_to_sec(block_frames)
                    ));
                } else {
                    xml_resources.push_str(&format!(
                        r#"        <asset id="v_{i}" name="{name}" src="{uri}" start="0s" duration="{duration}" hasVideo="1" format="r1"/>
"#,
                        i = i, name = xml_escape(&v.name), uri = file_to_uri(&v.path), duration = frame_to_sec(v_frames)
                    ));
                    
                    let dur_frames = v_frames.min(block_frames);
                    xml_anchors.push_str(&format!(
                        r#"                <asset-clip name="{name}" ref="v_{i}" offset="{offset}" start="0s" duration="{duration}" lane="1" videoRole="video"/>
"#,
                        name = xml_escape(&v.name), i = i, offset = offset_str, duration = frame_to_sec(dur_frames)
                    ));
                }
            }
            total_frames += block_frames;
        }

        let xml_spine = format!(
            r#"            <gap name="Master_Timeline" offset="0s" start="0s" duration="{total_duration}">
{anchors}            </gap>"#,
            total_duration = frame_to_sec(total_frames),
            anchors = xml_anchors
        );

        let fcpxml_content = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE fcpxml>
<fcpxml version="1.9">
    <resources>
{}    </resources>
    <library>
        <event name="AI自动化生成集">
            <project name="AI一键排版序列">
                <sequence format="r1" duration="{}" tcStart="0s" tcFormat="NDF" audioLayout="stereo" audioRate="48k">
                    <spine>
{}
                    </spine>
                </sequence>
            </project>
        </event>
    </library>
</fcpxml>"#,
            xml_resources, frame_to_sec(total_frames), xml_spine
        );

        fs::write(&export_path, fcpxml_content).map_err(|e| format!("导出 XML 失败: {}", e))?;
        return Ok(export_path.to_string_lossy().into_owned());
    }

    // ========================================================
    // 剪映安全草稿沙盒注入
    // ========================================================
    let draft_root = get_jianying_draft_dir()?;
    let folder_name = format!("AI自动对齐_{}", gen_id().chars().take(6).collect::<String>());
    let draft_path = draft_root.join(&folder_name);

    let image_dir = draft_path.join("image");
    let video_dir = draft_path.join("video");
    let audio_dir = draft_path.join("audio");

    fs::create_dir_all(&image_dir).map_err(|e| e.to_string())?;
    fs::create_dir_all(&video_dir).map_err(|e| e.to_string())?;
    fs::create_dir_all(&audio_dir).map_err(|e| e.to_string())?;

    let mut videos_list = Vec::new();
    let mut audios_list = Vec::new();
    let mut video_segments = Vec::new();
    let mut audio_segments = Vec::new();
    let mut meta_video_list = Vec::new();
    let mut meta_audio_list = Vec::new();
    let mut speeds_list = Vec::new(); 
    let mut current_time: i64 = 0;

    for pair in pairs {
        if pair.video.is_none() && pair.audio.is_none() { continue; }
        let video_duration = pair.video.as_ref().map_or(0, |v| get_media_duration(Path::new(&v.path)));
        let audio_duration = pair.audio.as_ref().map_or(0, |a| get_media_duration(Path::new(&a.path)));

        let block_duration = if audio_duration > 0 { audio_duration } else { video_duration };
        let start_time = current_time;

        if let Some(video) = &pair.video {
            let material_id = gen_id();
            let segment_id = gen_id();
            let is_photo = video.kind == "image";
            
            let target_dir = if is_photo { &image_dir } else { &video_dir };
            let target_path = target_dir.join(&video.name);
            link_media(Path::new(&video.path), &target_path)?;

            let target_duration = if is_photo { block_duration } else { video_duration.min(block_duration) };

            videos_list.push(build_video_material(&material_id, &target_path, &video.name, video_duration, is_photo));
            meta_video_list.push(json!({
                "id": material_id, "material_name": video.name,
                "path": target_path.to_string_lossy().replace("\\", "/"),
                "type": if is_photo { "photo" } else { "video" },
                "duration": video_duration, "width": CANVAS_W, "height": CANVAS_H
            }));
            
            video_segments.push(json!({
                "id": segment_id, "material_id": material_id,
                "target_timerange": { "start": start_time, "duration": target_duration },
                "source_timerange": { "start": 0, "duration": target_duration },
                "speed": 1.0, "volume": 0.0, "visible": true, "reverse": false,
                "clip": { "alpha": 1.0, "rotation": 0.0, "scale": { "x": 1.0, "y": 1.0 }, "transform": { "x": 0.0, "y": 0.0 }, "flip": { "horizontal": false, "vertical": false } },
                "render_index": 0, "track_render_index": 0, "track_attribute": 0,
                "extra_material_refs": [], "common_keyframes": [], "keyframe_refs": [], "is_photo": is_photo
            }));

            if !is_photo && target_duration < block_duration {
                let gap_duration = block_duration - target_duration;
                let freeze_mat_id = gen_id();
                let freeze_seg_id = gen_id();
                let speed_id = gen_id();

                videos_list.push(build_video_material(&freeze_mat_id, &target_path, &video.name, video_duration, false));
                meta_video_list.push(json!({
                    "id": freeze_mat_id, "material_name": video.name,
                    "path": target_path.to_string_lossy().replace("\\", "/"),
                    "type": "video",
                    "duration": video_duration, "width": CANVAS_W, "height": CANVAS_H
                }));

                let mut speed_val = 33_333.0 / gap_duration as f64;
                if speed_val < 0.01 { speed_val = 0.01; }
                let required_source_dur = (gap_duration as f64 * speed_val) as i64;
                
                speeds_list.push(json!({
                    "curve_speed": null, "id": speed_id.clone(), "mode": 0, "speed": speed_val, "type": "speed"
                }));

                let freeze_source_start = if video_duration > required_source_dur { video_duration - required_source_dur } else { 0 };
                
                video_segments.push(json!({
                    "id": freeze_seg_id, "material_id": freeze_mat_id,
                    "target_timerange": { "start": start_time + target_duration, "duration": gap_duration },
                    "source_timerange": { "start": freeze_source_start, "duration": required_source_dur },
                    "speed": speed_val, "volume": 0.0, "visible": true, "reverse": false,
                    "clip": { "alpha": 1.0, "rotation": 0.0, "scale": { "x": 1.0, "y": 1.0 }, "transform": { "x": 0.0, "y": 0.0 }, "flip": { "horizontal": false, "vertical": false } },
                    "render_index": 0, "track_render_index": 0, "track_attribute": 0,
                    "extra_material_refs": [speed_id], "common_keyframes": [], "keyframe_refs": [], "is_photo": false
                }));
            }
        }

        if let Some(audio) = &pair.audio {
            let material_id = gen_id();
            let segment_id = gen_id();
            let target_path = audio_dir.join(&audio.name);
            link_media(Path::new(&audio.path), &target_path)?;

            audios_list.push(build_audio_material(&material_id, &target_path, &audio.name, audio_duration));
            meta_audio_list.push(json!({
                "id": material_id, "name": audio.name,
                "path": target_path.to_string_lossy().replace("\\", "/"),
                "duration": audio_duration, "material_name": audio.name
            }));
            
            audio_segments.push(json!({
                "id": segment_id, "material_id": material_id,
                "target_timerange": { "start": start_time, "duration": audio_duration },
                "source_timerange": { "start": 0, "duration": audio_duration },
                "render_timerange": { "start": 0, "duration": 0 },
                "desc": "", "state": 0, "speed": 1.0, "is_loop": false,
                "is_tone_modify": false, "reverse": false, "intensifies_audio": false,
                "cartoon": false, "volume": 1.0, "last_nonzero_volume": 1.0,
                "clip": null, "uniform_scale": null, "extra_material_refs": []
            }));
        }
        current_time += block_duration;
    }

    let meta_json = build_meta_json(&draft_path, &folder_name, meta_audio_list, meta_video_list);
    fs::write(draft_path.join("draft_meta_info.json"), serde_json::to_string_pretty(&meta_json).unwrap()).map_err(|e| e.to_string())?;

    let content_json = json!({
        "id": gen_id(), "name": folder_name, "duration": current_time, "fps": FPS,
        "canvas_config": { "width": CANVAS_W, "height": CANVAS_H, "ratio": "original" },
        "platform": { "app_source": "lv", "app_version": "", "os": if cfg!(target_os = "macos") { "mac" } else { "windows" } },
        "config": { "adjust_max_index": 1, "attachment_info": [], "combination_max_index": 1, "export_range": null, "extract_audio_last_index": 1, "lyrics_recognition_id": "", "os": if cfg!(target_os = "macos") { "mac" } else { "windows" } },
        "cover": { "format": "jpeg", "height": CANVAS_H, "path": "", "width": CANVAS_W },
        "materials": {
            "videos": videos_list, "audios": audios_list, "audio_effects": [], "audio_fades": [], "audio_balances": [],
            "audio_track_indexes": [], "beats": [], "canvases": [], "texts": [], "transitions": [], "effects": [],
            "stickers": [], "video_effects": [], "material_animations": [], "filters": [], "masks": [], 
            "speeds": speeds_list,
            "sound_channel_mappings": [], "vocal_separations": [], "smart_crops": [], "manual_deformations": []
        },
        "mutable_config": { "align_canvas_mode": 0, "bgm_sync": true },
        "tracks": [
            { "id": gen_id(), "type": "video", "name": "主视频轨道", "is_default_name": true, "attribute": 0, "flag": 0, "render_index": 0, "segments": video_segments },
            { "id": gen_id(), "type": "audio", "name": "独立音频轨道", "is_default_name": true, "attribute": 0, "flag": 0, "render_index": 0, "mute": false, "segments": audio_segments }
        ],
        "version": DRAFT_VERSION
    });

    fs::write(draft_path.join("draft_info.json"), serde_json::to_string_pretty(&content_json).unwrap()).map_err(|e| e.to_string())?;

    #[cfg(target_os = "macos")]
    let _ = Command::new("open").arg(&draft_path).spawn();

    Ok(draft_path.to_string_lossy().into_owned())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![smart_sniff_folder, generate_draft])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}