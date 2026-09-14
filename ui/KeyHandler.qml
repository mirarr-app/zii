import QtQuick

Item {
    id: root

    property string mode: "NORMAL" // "NORMAL" or "EDIT"
    property bool cropActive: false
    property bool adjustActive: false
    property bool saveDialogActive: false

    // Signals emitted for actions
    signal navigatePrev()
    signal navigateNext()
    signal pan(real dx, real dy)
    signal zoomIn()
    signal zoomOut()
    signal zoomReset()
    signal zoom100()
    signal toggleFullscreen()
    signal deleteCurrent(bool permanent)
    signal restoreTrash()
    signal enterEditMode()
    signal exitEditMode()
    signal quit()

    // Edit actions
    signal toggleCrop()
    signal applyCrop()
    signal cancelCrop()
    signal cropMove(real dx, real dy)
    signal cropResize(real dw, real dh)

    signal rotateCW()
    signal rotateCCW()
    signal flipH()
    signal flipV()
    signal toggleAdjust()
    signal adjustStep(int delta)
    signal undo()
    signal redo()
    signal saveOverwrite()
    signal savePrompt()
    signal saveCopy()
    signal cancelSaveDialog()

    // Double 'd' sequence tracker for 'dd' delete in Normal mode
    property int lastDTime: 0

    function handleKeyEvent(event) {
        var key = event.key;
        var text = event.text;
        var modifiers = event.modifiers;
        var hasCtrl = (modifiers & Qt.ControlModifier) !== 0;
        var hasShift = (modifiers & Qt.ShiftModifier) !== 0;

        // Save dialog intercepts keys first
        if (root.saveDialogActive) {
            if (key === Qt.Key_Escape) {
                root.cancelSaveDialog();
                event.accepted = true;
                return;
            }
            if (key === Qt.Key_W || text === "w") {
                root.saveOverwrite();
                event.accepted = true;
                return;
            }
            if (key === Qt.Key_S || text === "s") {
                root.saveCopy();
                event.accepted = true;
                return;
            }
            return;
        }

        // ================= EDIT MODE =================
        if (root.mode === "EDIT") {
            // Esc key in edit mode
            if (key === Qt.Key_Escape) {
                if (root.cropActive) {
                    root.cancelCrop();
                } else if (root.adjustActive) {
                    root.toggleAdjust();
                } else {
                    root.exitEditMode();
                }
                event.accepted = true;
                return;
            }

            // In crop sub-mode: arrow keys move or resize crop box
            if (root.cropActive) {
                if (key === Qt.Key_Return || key === Qt.Key_Enter) {
                    root.applyCrop();
                    event.accepted = true;
                    return;
                }
                var step = hasShift ? 20 : 10;
                if (key === Qt.Key_Left) {
                    if (hasShift) root.cropResize(-step, 0); else root.cropMove(-step, 0);
                    event.accepted = true; return;
                }
                if (key === Qt.Key_Right) {
                    if (hasShift) root.cropResize(step, 0); else root.cropMove(step, 0);
                    event.accepted = true; return;
                }
                if (key === Qt.Key_Up) {
                    if (hasShift) root.cropResize(0, -step); else root.cropMove(0, -step);
                    event.accepted = true; return;
                }
                if (key === Qt.Key_Down) {
                    if (hasShift) root.cropResize(0, step); else root.cropMove(0, step);
                    event.accepted = true; return;
                }
            }

            // Adjustments panel bracket keys [ and ]
            if (root.adjustActive) {
                if (text === "[") {
                    root.adjustStep(-5);
                    event.accepted = true;
                    return;
                }
                if (text === "]") {
                    root.adjustStep(5);
                    event.accepted = true;
                    return;
                }
            }

            // Undo / Redo
            if (hasCtrl && (key === Qt.Key_R || text === "r")) {
                root.redo();
                event.accepted = true;
                return;
            }
            if (!hasCtrl && text === "u") {
                root.undo();
                event.accepted = true;
                return;
            }

            // Save keys
            if (text === "w") {
                root.saveOverwrite();
                event.accepted = true;
                return;
            }
            if (text === "s") {
                root.savePrompt();
                event.accepted = true;
                return;
            }

            // Edit tool toggles
            if (text === "c") {
                root.toggleCrop();
                event.accepted = true;
                return;
            }
            if (text === "r") {
                root.rotateCW();
                event.accepted = true;
                return;
            }
            if (text === "R") {
                root.rotateCCW();
                event.accepted = true;
                return;
            }
            if (text === "h") {
                root.flipH();
                event.accepted = true;
                return;
            }
            if (text === "v") {
                root.flipV();
                event.accepted = true;
                return;
            }
            if (text === "a") {
                root.toggleAdjust();
                event.accepted = true;
                return;
            }

            return;
        }

        // ================= NORMAL MODE =================
        // Fullscreen toggle
        if (key === Qt.Key_F || key === Qt.Key_F11) {
            root.toggleFullscreen();
            event.accepted = true;
            return;
        }

        // Quit
        if (text === "q" || key === Qt.Key_Escape) {
            root.quit();
            event.accepted = true;
            return;
        }

        // Enter edit mode
        if (text === "i") {
            root.enterEditMode();
            event.accepted = true;
            return;
        }

        // Navigation
        if (text === "h" || key === Qt.Key_Left) {
            root.navigatePrev();
            event.accepted = true;
            return;
        }
        if (text === "l" || key === Qt.Key_Right) {
            root.navigateNext();
            event.accepted = true;
            return;
        }
        if (text === "j" || key === Qt.Key_Down) {
            root.pan(0, -40);
            event.accepted = true;
            return;
        }
        if (text === "k" || key === Qt.Key_Up) {
            root.pan(0, 40);
            event.accepted = true;
            return;
        }

        // Zoom keys
        if (text === "+" || text === "=" || text === "z") {
            root.zoomIn();
            event.accepted = true;
            return;
        }
        if (text === "-" || text === "_" || text === "Z") {
            root.zoomOut();
            event.accepted = true;
            return;
        }
        if (text === "0") {
            root.zoomReset();
            event.accepted = true;
            return;
        }
        if (text === "1") {
            root.zoom100();
            event.accepted = true;
            return;
        }

        // Deletion: 'dd' (double d) or 'Shift+D' (permanent)
        if (text === "D" && hasShift) {
            root.deleteCurrent(true);
            event.accepted = true;
            return;
        }
        if (text === "d") {
            var now = Date.now();
            if (now - root.lastDTime < 500) {
                root.deleteCurrent(false);
                root.lastDTime = 0;
            } else {
                root.lastDTime = now;
            }
            event.accepted = true;
            return;
        }

        // Undo delete / restore
        if (text === "u") {
            root.restoreTrash();
            event.accepted = true;
            return;
        }
    }
}
