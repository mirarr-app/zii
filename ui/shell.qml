import QtQuick
import Quickshell
import Quickshell.Io

ShellRoot {
    id: root

    Theme {
        id: theme
    }

    // Direct watcher on Omarchy colors.toml for real-time live theme reload
    FileView {
        id: colorsWatcher
        path: Quickshell.env("HOME") + "/.local/state/omarchy/current/theme/colors.toml"
        watchChanges: true
        printErrors: false
        onLoaded: {
            parseColors(text());
        }
        onFileChanged: reload()
    }

    function parseColors(raw) {
        var lines = String(raw || "").split("\n");
        var obj = {};
        for (var i = 0; i < lines.length; i++) {
            var m = lines[i].match(/^\s*([A-Za-z0-9_-]+)\s*=\s*["']?(#[0-9A-Fa-f]{6})/);
            if (m) obj[m[1]] = m[2];
        }
        theme.apply(obj);
    }

    // Socket connection to Rust backend
    Socket {
        id: socket
        path: Quickshell.env("ZII_SOCKET") || "/tmp/zii.sock"
        connected: true

        parser: SplitParser {
            splitMarker: "\n"
            onRead: data => {
                var line = data.trim();
                if (!line) return;
                try {
                    var msg = JSON.parse(line);
                    root.handleServerEvent(msg);
                } catch (e) {
                    console.error("Failed to parse server message:", e, line);
                }
            }
        }

        onConnectedChanged: {
            if (connected) {
                root.sendIpc({ type: "ready" });
            }
        }
        Component.onCompleted: {
            connected = true;
            if (connected) {
                root.sendIpc({ type: "ready" });
            }
        }
    }

    Timer {
        id: readyRetry
        interval: 150
        repeat: true
        running: !root.currentEntry
        onTriggered: {
            if (!socket.connected) {
                socket.connected = true;
            } else {
                root.sendIpc({ type: "ready" });
            }
        }
    }

    function sendIpc(req) {
        if (socket && socket.connected) {
            socket.write(JSON.stringify(req) + "\n");
            socket.flush();
        }
    }

    // State
    property string currentMode: "NORMAL" // "NORMAL" or "EDIT"
    property var currentEntry: null
    property int currentIndex: 0
    property int totalCount: 0
    property string activeImagePath: ""
    property real displayWidth: 0
    property real displayHeight: 0
    property bool canUndo: false
    property bool canRedo: false
    property bool isFullscreen: false

    function handleServerEvent(msg) {
        if (!msg || !msg.type) return;

        if (msg.type === "theme") {
            theme.apply(msg.theme);
        } else if (msg.type === "directory_state") {
            root.currentIndex = msg.index || 0;
            root.totalCount = msg.total || 0;
            root.currentEntry = msg.current || null;
            if (root.currentEntry && root.currentMode === "NORMAL") {
                root.activeImagePath = root.currentEntry.path;
                root.displayWidth = root.currentEntry.width;
                root.displayHeight = root.currentEntry.height;
            }
            hud.wake();
        } else if (msg.type === "edit_state") {
            root.canUndo = !!msg.can_undo;
            root.canRedo = !!msg.can_redo;
            if (msg.active) {
                root.currentMode = "EDIT";
                if (msg.preview_path) {
                    root.activeImagePath = msg.preview_path;
                }
                if (msg.width > 0) root.displayWidth = msg.width;
                if (msg.height > 0) root.displayHeight = msg.height;
            } else {
                root.currentMode = "NORMAL";
                cropOverlay.active = false;
                adjPanel.active = false;
                saveDlg.active = false;
                if (root.currentEntry) {
                    root.activeImagePath = root.currentEntry.path;
                    root.displayWidth = root.currentEntry.width;
                    root.displayHeight = root.currentEntry.height;
                }
            }
            hud.wake();
        } else if (msg.type === "toast") {
            toast.show(msg.message, msg.level);
        } else if (msg.type === "close") {
            Qt.quit();
        }
    }

    // Main window
    FloatingWindow {
        id: win
        visible: true
        fullscreen: root.isFullscreen
        title: (root.currentMode === "EDIT" ? "[EDIT] " : "") + (root.currentEntry ? root.currentEntry.filename : "Zii Photo Viewer")
        implicitWidth: 1280
        implicitHeight: 820
        color: theme.darkerBackground

        Item {
            anchors.fill: parent
            focus: true
            Component.onCompleted: forceActiveFocus()

            Keys.onPressed: event => {
                keyHandler.handleKeyEvent(event);
            }

            // Mouse tracking to wake up HUD
            MouseArea {
                anchors.fill: parent
                hoverEnabled: true
                propagateComposedEvents: true
                onPositionChanged: {
                    hud.wake();
                    mouse.accepted = false;
                }
            }

            // Pan and zoom image viewport
            ImageViewport {
                id: canvas
                anchors.fill: parent
                theme: theme
                source: root.activeImagePath
                imgNaturalWidth: root.displayWidth
                imgNaturalHeight: root.displayHeight

                onInteractionOccurred: {
                    hud.wake();
                }
            }

            // Interactive Crop Overlay
            CropOverlay {
                id: cropOverlay
                anchors.fill: parent
                theme: theme
                imgNaturalWidth: canvas.imgNaturalWidth
                imgNaturalHeight: canvas.imgNaturalHeight
                panX: canvas.panX
                panY: canvas.panY
                zoomFactor: canvas.zoomFactor
                active: false

                onCropApplied: (x, y, w, h) => {
                    root.sendIpc({
                        type: "edit_crop",
                        x: Math.round(x),
                        y: Math.round(y),
                        width: Math.round(w),
                        height: Math.round(h)
                    });
                    cropOverlay.active = false;
                }

                onCropCanceled: {
                    cropOverlay.active = false;
                }
            }

            // Adjustments Panel (Brightness / Contrast)
            AdjustmentsPanel {
                id: adjPanel
                theme: theme
                anchors.top: parent.top
                anchors.topMargin: 70
                anchors.right: parent.right
                anchors.rightMargin: 24
                active: false

                onAdjustmentsApplied: (bright, cont) => {
                    root.sendIpc({
                        type: "edit_adjust",
                        brightness: bright,
                        contrast: cont
                    });
                }
            }

            // Edit Toolbar (visible in Edit mode)
            EditToolbar {
                id: editToolbar
                theme: theme
                anchors.top: parent.top
                anchors.topMargin: 16
                anchors.horizontalCenter: parent.horizontalCenter
                active: root.currentMode === "EDIT"
                canUndo: root.canUndo
                canRedo: root.canRedo
                cropActive: cropOverlay.active
                adjustActive: adjPanel.active

                onTriggerCrop: {
                    cropOverlay.active = !cropOverlay.active;
                }
                onTriggerRotateCW: {
                    root.sendIpc({ type: "edit_rotate", degrees: 90 });
                }
                onTriggerRotateCCW: {
                    root.sendIpc({ type: "edit_rotate", degrees: 270 });
                }
                onTriggerFlipH: {
                    root.sendIpc({ type: "edit_flip", horizontal: true, vertical: false });
                }
                onTriggerFlipV: {
                    root.sendIpc({ type: "edit_flip", horizontal: false, vertical: true });
                }
                onTriggerAdjust: {
                    adjPanel.active = !adjPanel.active;
                }
                onTriggerUndo: {
                    root.sendIpc({ type: "edit_undo" });
                }
                onTriggerRedo: {
                    root.sendIpc({ type: "edit_redo" });
                }
                onTriggerSaveOverwrite: {
                    root.sendIpc({ type: "edit_save", overwrite: true, filename: null });
                }
                onTriggerSaveCopy: {
                    saveDlg.active = true;
                }
                onTriggerExit: {
                    root.sendIpc({ type: "edit_cancel" });
                }
            }

            // Save choice dialog modal
            SaveDialog {
                id: saveDlg
                theme: theme
                active: false

                onSaveOverwrite: {
                    saveDlg.active = false;
                    root.sendIpc({ type: "edit_save", overwrite: true, filename: null });
                }
                onSaveCopy: {
                    saveDlg.active = false;
                    root.sendIpc({ type: "edit_save", overwrite: false, filename: null });
                }
                onCancel: {
                    saveDlg.active = false;
                }
            }

            // Toast notifications at top
            Toast {
                id: toast
                anchors.horizontalCenter: parent.horizontalCenter
                anchors.top: parent.top
                anchors.topMargin: root.currentMode === "EDIT" ? 72 : 18
            }

            // Auto-hiding Minimalist HUD at bottom
            HudOverlay {
                id: hud
                theme: theme
                anchors.bottom: parent.bottom
                anchors.bottomMargin: 18
                anchors.horizontalCenter: parent.horizontalCenter
                mode: root.currentMode
                filename: root.currentEntry ? root.currentEntry.filename : ""
                imgWidth: Math.round(root.displayWidth)
                imgHeight: Math.round(root.displayHeight)
                fileSize: root.currentEntry ? root.currentEntry.file_size : 0
                zoomFactor: canvas.zoomFactor
                currentIndex: root.currentIndex
                totalCount: root.totalCount
            }

            // Help overlay modal (toggled by '?')
            HelpOverlay {
                id: helpOverlay
                theme: theme
                mode: root.currentMode
                cropActive: cropOverlay.active
                adjustActive: adjPanel.active
                active: false

                onClosed: {
                    helpOverlay.active = false;
                }
            }

            // Keyboard Dispatcher
            KeyHandler {
                id: keyHandler
                mode: root.currentMode
                cropActive: cropOverlay.active
                adjustActive: adjPanel.active
                saveDialogActive: saveDlg.active
                helpActive: helpOverlay.active

                onToggleHelp: {
                    helpOverlay.active = !helpOverlay.active;
                }

                onNavigateNext: {
                    root.sendIpc({ type: "navigate", direction: "next", target: null });
                }
                onNavigatePrev: {
                    root.sendIpc({ type: "navigate", direction: "prev", target: null });
                }
                onPan: (dx, dy) => {
                    canvas.panBy(dx, dy);
                }
                onZoomIn: canvas.zoomIn()
                onZoomOut: canvas.zoomOut()
                onZoomReset: canvas.resetView()
                onZoom100: canvas.setZoom100()

                onToggleFullscreen: {
                    // Toggle window maximized/fullscreen
                    root.isFullscreen = !root.isFullscreen;
                }

                onDeleteCurrent: perm => {
                    root.sendIpc({ type: "delete_current", permanent: perm });
                }
                onRestoreTrash: {
                    root.sendIpc({ type: "restore_trash" });
                }
                onEnterEditMode: {
                    root.sendIpc({ type: "edit_start" });
                }
                onExitEditMode: {
                    root.sendIpc({ type: "edit_cancel" });
                }
                onQuit: {
                    root.sendIpc({ type: "quit" });
                }

                // Edit shortcuts
                onToggleCrop: {
                    cropOverlay.active = !cropOverlay.active;
                }
                onApplyCrop: cropOverlay.apply()
                onCancelCrop: cropOverlay.cancel()
                onCropMove: (dx, dy) => {
                    if (dx > 0) cropOverlay.moveRight(dx);
                    else if (dx < 0) cropOverlay.moveLeft(-dx);
                    if (dy > 0) cropOverlay.moveDown(dy);
                    else if (dy < 0) cropOverlay.moveUp(-dy);
                }
                onCropResize: (dw, dh) => {
                    if (dw > 0) cropOverlay.expandW(dw);
                    else if (dw < 0) cropOverlay.shrinkW(-dw);
                    if (dh > 0) cropOverlay.expandH(dh);
                    else if (dh < 0) cropOverlay.shrinkH(-dh);
                }

                onRotateCW: {
                    root.sendIpc({ type: "edit_rotate", degrees: 90 });
                }
                onRotateCCW: {
                    root.sendIpc({ type: "edit_rotate", degrees: 270 });
                }
                onFlipH: {
                    root.sendIpc({ type: "edit_flip", horizontal: true, vertical: false });
                }
                onFlipV: {
                    root.sendIpc({ type: "edit_flip", horizontal: false, vertical: true });
                }
                onToggleAdjust: {
                    adjPanel.active = !adjPanel.active;
                }
                onAdjustStep: delta => {
                    adjPanel.stepBrightness(delta);
                }
                onUndo: {
                    root.sendIpc({ type: "edit_undo" });
                }
                onRedo: {
                    root.sendIpc({ type: "edit_redo" });
                }
                onSaveOverwrite: {
                    root.sendIpc({ type: "edit_save", overwrite: true, filename: null });
                }
                onSavePrompt: {
                    saveDlg.active = true;
                }
                onSaveCopy: {
                    saveDlg.active = false;
                    root.sendIpc({ type: "edit_save", overwrite: false, filename: null });
                }
                onCancelSaveDialog: {
                    saveDlg.active = false;
                }
            }
        }
    }
}
