\# Deadlock Discord Rich Presence



A standalone Discord Rich Presence application for \*\*Deadlock\*\*, featuring hero portraits and live game status.



\## Features



\- Displays your current Deadlock game status on Discord.

\- Shows hero portraits using Discord Rich Presence assets.

\- Detects whether Deadlock is running.

\- Automatically detects common Steam installation locations.

\- Runs quietly in the Windows system tray.

\- Includes tray menu options for opening settings and quitting the application.

\- Writes application logs to a file for troubleshooting.



\## Installation



1\. Download the latest release ZIP from the \[GitHub Releases](https://github.com/k18554540/deadlock-discord-rich-presence/releases) page.

2\. Extract the ZIP file into a folder of your choice.

3\. Run `deadlock-discord-presence.exe`.

4\. Make sure Discord is running.

5\. Launch Deadlock through the application if it is not already running.



The application will attempt to locate your Deadlock installation automatically.



If automatic detection cannot find your installation, make sure your Steam library contains:



`steamapps/common/Deadlock/game/citadel`



\## System Tray



After launching, the application runs in the Windows system tray.



Right-click the tray icon to access:



\- \*\*Deadlock Rich Presence: Running\*\* — Shows the application status.

\- \*\*Open Settings\*\* — Opens the configuration file.

\- \*\*Quit\*\* — Closes the application.



\## Settings



The application creates a `settings.json` file beside the executable on first launch.



You can open it through the system tray menu using \*\*Open Settings\*\*.



Changes made to the configuration file take effect the next time the application is launched.



\## Logs



The application writes logs to:



`deadlock-discord-presence.log`



The log file is located beside the executable and can be used to troubleshoot issues.



\## Requirements



\- Windows

\- Steam

\- Deadlock

\- Discord desktop application



\## Building from Source



Make sure you have the Rust toolchain installed.



Clone the repository:



```powershell

git clone https://github.com/k18554540/deadlock-discord-rich-presence.git

cd deadlock-discord-rich-presence

```



Build the release executable:



```powershell

cargo build --release

```



The executable will be generated in:



`target/release/deadlock-discord-presence.exe`



Copy `icon.ico` into the same directory as the executable before running it.



\## License



This project is licensed under the GNU General Public License v3.0. See \[LICENSE](LICENSE) for details.

