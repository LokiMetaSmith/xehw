
`cargo run --release`

On Linux you need to first run:

`sudo apt-get install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libssl-dev`

## Collaborative Coding

This project supports networked collaborative coding. To use it:

1.  Start the broadcast server:
    ```bash
    pip install websockets
    python3 server/broadcast.py
    ```
2.  Run the application (multiple instances):
    ```bash
    cargo run --release
    ```
3.  In the app, go to **Network > Connection...**
4.  Enter the server URL (default: `ws://localhost:8080`) and click **Connect**.
5.  Edits to the code and agent activities will be synchronized between connected peers.

## Command Palette

Press `Cmd+Shift+P` (macOS) or `Ctrl+Shift+P` (Windows/Linux) to open the Command Palette.
This allows quick access to various actions like running code, toggling views, and managing agents.

Screenshots:

![xehimg.png](imgs/xehimg.png)

![doomfire.gif](imgs/doomfire.gif)

![rollback.gif](imgs/rollback.gif)

![gbtile.gif](imgs/gbtile.gif)

![reversestep.gif](imgs/reversestep.gif)

![freeze.gif](imgs/freeze.gif)
