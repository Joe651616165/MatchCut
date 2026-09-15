## 🍏 macOS 用户安装必看：解除“已损坏”提示

如果您在 macOS 系统上首次打开 MatchCut 时遇到以下报错提示：
> **“MatchCut”已损坏，无法打开。你应该将它移到废纸篓。**

⚠️ **请放心，这并非软件损坏或含有病毒。**
这是 macOS 系统（Gatekeeper）针对未向苹果支付每年 99 美元开发者证书费用的开源独立软件，所采取的默认“安全隔离”机制。

**只需 3 步，10 秒钟即可轻松解锁并永久正常使用：**

1. 📦 **正确安装**：双击下载好的 `.dmg` 安装包，务必先将 `MatchCut` 图标拖入到 `Applications（应用程序）` 文件夹中。
2. 💻 **打开终端**：按键盘上的 `Command (⌘) + 空格` 呼出聚焦搜索，输入 `终端`（或 `Terminal`）并回车打开。
3. 🛠️ **一键解锁**：在终端中复制并粘贴以下命令，然后按回车键执行：

```bash
xattr -cr /Applications/MatchCut.app






## 🍏 A must-see for macOS users during installation: Removing the “Corrupted” prompt

If you encounter the following error message when you open MatchCut for the first time on macOS:
> **"MatchCut" is corrupted and cannot be opened. You should move it to the Trash. **

⚠️ **Please rest assured that this is not a corrupted software or a virus. **
This is the default "security isolation" mechanism adopted by the macOS system (Gatekeeper) for open source independent software that does not pay Apple the $99 annual developer certificate fee.

**Just 3 steps, 10 seconds to easily unlock and use permanently:**

1. 📦 **Correct installation**: Double-click the downloaded `.dmg` installation package, and be sure to drag the `MatchCut` icon into the `Applications` folder first.
2. 💻 **Open Terminal**: Press `Command (⌘) + Space` on the keyboard to call out the focus search, enter `Terminal` (or `Terminal`) and press Enter to open it.
3. 🛠️ **One-click unlock**: Copy and paste the following command in the terminal, and then press Enter to execute:

```bash
xattr -cr /Applications/MatchCut.app