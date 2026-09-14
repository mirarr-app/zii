import QtQuick

Rectangle {
    id: root

    required property var theme
    property bool active: false
    property string mode: "NORMAL" // "NORMAL" or "EDIT"
    property bool cropActive: false
    property bool adjustActive: false

    signal closed()

    visible: active
    opacity: active ? 1.0 : 0.0
    scale: active ? 1.0 : 0.96
    Behavior on opacity { NumberAnimation { duration: 180; easing.type: Easing.OutCubic } }
    Behavior on scale { NumberAnimation { duration: 180; easing.type: Easing.OutBack } }

    anchors.fill: parent
    color: Qt.rgba(0, 0, 0, 0.65)

    MouseArea {
        anchors.fill: parent
        onClicked: root.closed()
    }

    Rectangle {
        id: modalCard
        anchors.centerIn: parent
        width: Math.min(840, parent.width - 48)
        height: Math.min(620, parent.height - 48)
        radius: 16
        color: Qt.rgba(theme.darkerBackground.r, theme.darkerBackground.g, theme.darkerBackground.b, 0.96)
        border.color: theme.accent
        border.width: 1.5

        MouseArea {
            anchors.fill: parent // Prevent clicks inside modal from closing it
        }

        Column {
            anchors.fill: parent
            anchors.margins: 26
            spacing: 20

            // Header Bar
            Item {
                width: parent.width
                height: 28

                Row {
                    anchors.left: parent.left
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 12

                    Image {
                        width: 24
                        height: 24
                        source: "assets/zii.svg"
                        sourceSize.width: 48
                        sourceSize.height: 48
                        anchors.verticalCenter: parent.verticalCenter
                    }

                    Rectangle {
                        width: modeLabel.implicitWidth + 16
                        height: 24
                        radius: 12
                        color: root.mode === "EDIT" ? theme.accent : theme.surface
                        border.color: root.mode === "EDIT" ? theme.accent : theme.muted
                        border.width: 1
                        anchors.verticalCenter: parent.verticalCenter

                        Text {
                            id: modeLabel
                            anchors.centerIn: parent
                            text: root.mode
                            color: root.mode === "EDIT" ? theme.darkerBackground : theme.foreground
                            font.family: theme.fontFamily
                            font.pixelSize: 11
                            font.bold: true
                        }
                    }

                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        text: {
                            if (root.mode === "EDIT") {
                                if (root.cropActive) return "Keyboard Shortcuts — Crop Tool";
                                if (root.adjustActive) return "Keyboard Shortcuts — Adjustments";
                                return "Keyboard Shortcuts — Photo Editor";
                            }
                            return "Keyboard Shortcuts — Photo Viewer";
                        }
                        color: theme.brightForeground
                        font.family: theme.fontFamily
                        font.pixelSize: 17
                        font.bold: true
                    }
                }

                Text {
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    text: "Press ? or Esc to close"
                    color: theme.darkForeground
                    font.family: theme.fontFamily
                    font.pixelSize: 11
                }
            }

            // Separator line
            Rectangle {
                width: parent.width
                height: 1
                color: Qt.rgba(theme.muted.r, theme.muted.g, theme.muted.b, 0.25)
            }

            // Shortcut columns container (Flickable for smaller screens)
            Flickable {
                width: parent.width
                height: parent.height - 90
                contentWidth: width
                contentHeight: columnsRow.implicitHeight
                clip: true

                Row {
                    id: columnsRow
                    width: parent.width
                    spacing: 28

                    // Column 1
                    Column {
                        width: (parent.width - 28) / 2
                        spacing: 18

                        // Section 1 Header
                        Text {
                            text: root.mode === "EDIT" ? "TOOLS & TRANSFORMS" : "NAVIGATION & PAN"
                            color: theme.accent
                            font.family: theme.fontFamily
                            font.pixelSize: 12
                            font.bold: true
                            font.letterSpacing: 1.0
                        }

                        Column {
                            width: parent.width
                            spacing: 8

                            Repeater {
                                model: root.mode === "EDIT" ? editToolsModel : navShortcutsModel
                                delegate: ShortcutRow {
                                    width: parent.width
                                    theme: root.theme
                                    keys: modelData.keys
                                    desc: modelData.desc
                                }
                            }
                        }

                        // Section 2 in Column 1 (for normal mode zoom)
                        Text {
                            text: root.mode === "EDIT" ? "HISTORY & SAVING" : "ZOOM & DISPLAY"
                            color: theme.accent
                            font.family: theme.fontFamily
                            font.pixelSize: 12
                            font.bold: true
                            font.letterSpacing: 1.0
                        }

                        Column {
                            width: parent.width
                            spacing: 8

                            Repeater {
                                model: root.mode === "EDIT" ? editSaveModel : zoomShortcutsModel
                                delegate: ShortcutRow {
                                    width: parent.width
                                    theme: root.theme
                                    keys: modelData.keys
                                    desc: modelData.desc
                                }
                            }
                        }
                    }

                    // Column 2
                    Column {
                        width: (parent.width - 28) / 2
                        spacing: 18

                        Text {
                            text: root.mode === "EDIT" ? "ACTIVE TOOL CONTROLS" : "ACTIONS & FILE OPS"
                            color: theme.accent
                            font.family: theme.fontFamily
                            font.pixelSize: 12
                            font.bold: true
                            font.letterSpacing: 1.0
                        }

                        Column {
                            width: parent.width
                            spacing: 8

                            Repeater {
                                model: root.mode === "EDIT" ? editControlsModel : actionsShortcutsModel
                                delegate: ShortcutRow {
                                    width: parent.width
                                    theme: root.theme
                                    keys: modelData.keys
                                    desc: modelData.desc
                                }
                            }
                        }

                        Text {
                            text: root.mode === "EDIT" ? "GENERAL" : "GENERAL & EXIT"
                            color: theme.accent
                            font.family: theme.fontFamily
                            font.pixelSize: 12
                            font.bold: true
                            font.letterSpacing: 1.0
                        }

                        Column {
                            width: parent.width
                            spacing: 8

                            Repeater {
                                model: root.mode === "EDIT" ? editGeneralModel : generalShortcutsModel
                                delegate: ShortcutRow {
                                    width: parent.width
                                    theme: root.theme
                                    keys: modelData.keys
                                    desc: modelData.desc
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Helper component for displaying a row: [Badge] [Description]
    component ShortcutRow: Row {
        id: sRow
        property var theme
        property var keys: []
        property string desc: ""
        spacing: 10

        Row {
            spacing: 4
            anchors.verticalCenter: parent.verticalCenter
            width: 130

            Repeater {
                model: sRow.keys
                delegate: Rectangle {
                    height: 22
                    width: Math.max(24, kbdText.implicitWidth + 10)
                    radius: 5
                    color: theme.surface
                    border.color: theme.muted
                    border.width: 1
                    anchors.verticalCenter: parent.verticalCenter

                    Text {
                        id: kbdText
                        anchors.centerIn: parent
                        text: modelData
                        color: theme.accent
                        font.family: theme.fontFamily
                        font.pixelSize: 11
                        font.bold: true
                    }
                }
            }
        }

        Text {
            anchors.verticalCenter: parent.verticalCenter
            text: sRow.desc
            color: theme.lightForeground
            font.family: theme.fontFamily
            font.pixelSize: 12
            elide: Text.ElideRight
            width: parent.width - 140
        }
    }

    // Data models for shortcuts
    // --- Normal Mode ---
    readonly property var navShortcutsModel: [
        { keys: ["h", "←"], desc: "Previous image in directory" },
        { keys: ["l", "→"], desc: "Next image in directory" },
        { keys: ["j", "↓"], desc: "Pan down (when zoomed in)" },
        { keys: ["k", "↑"], desc: "Pan up (when zoomed in)" },
        { keys: ["Drag"], desc: "Smooth mouse panning" }
    ]

    readonly property var zoomShortcutsModel: [
        { keys: ["+", "=", "z"], desc: "Zoom in towards center" },
        { keys: ["-", "_", "Z"], desc: "Zoom out from center" },
        { keys: ["0"], desc: "Reset zoom & fit to window" },
        { keys: ["1"], desc: "100% (1:1 original pixel scale)" },
        { keys: ["Wheel"], desc: "Focal zoom towards mouse pointer" },
        { keys: ["2x Click"], desc: "Toggle between Fit and 100%" }
    ]

    readonly property var actionsShortcutsModel: [
        { keys: ["Space"], desc: "Play / pause animated GIF or WebP" },
        { keys: ["i"], desc: "Enter Edit Mode" },
        { keys: ["e", "x"], desc: "Toggle EXIF metadata inspector" },
        { keys: ["y"], desc: "Copy image to clipboard (wl-copy)" },
        { keys: ["Y"], desc: "Copy image path to clipboard" },
        { keys: ["W"], desc: "Set as Omarchy desktop wallpaper" },
        { keys: ["dd"], desc: "Move photo directly to Trash" },
        { keys: ["u"], desc: "Undo last delete (restore from trash)" },
        { keys: ["Shift+D"], desc: "Permanently delete file" },
        { keys: ["f", "F11"], desc: "Toggle Fullscreen" }
    ]

    readonly property var generalShortcutsModel: [
        { keys: ["?"], desc: "Toggle keyboard shortcuts help" },
        { keys: ["q", "Esc"], desc: "Quit Zii" }
    ]

    // --- Edit Mode ---
    readonly property var editToolsModel: [
        { keys: ["c"], desc: "Toggle interactive Crop tool" },
        { keys: ["r"], desc: "Rotate 90° Clockwise" },
        { keys: ["R"], desc: "Rotate 90° Counter-Clockwise" },
        { keys: ["h"], desc: "Flip Horizontal" },
        { keys: ["v"], desc: "Flip Vertical" },
        { keys: ["a"], desc: "Toggle Adjustments (Bright/Contrast/Sat)" }
    ]

    readonly property var editControlsModel: [
        { keys: ["0 - 4"], desc: "Aspect ratio (0:Free, 1:1:1, 2:16:9, 3:4:3, 4:3:2)" },
        { keys: ["Arrows"], desc: "Move crop box (in Crop mode)" },
        { keys: ["Shift+Arr"], desc: "Resize crop box (enforces aspect ratio)" },
        { keys: ["Enter"], desc: "Apply crop to image" },
        { keys: ["[", "]"], desc: "Decrease / increase slider value" },
        { keys: ["Handles"], desc: "Drag 8 corner/edge resize handles" }
    ]

    readonly property var editSaveModel: [
        { keys: ["u"], desc: "Undo edit step" },
        { keys: ["Ctrl+r"], desc: "Redo edit step" },
        { keys: ["w"], desc: "Save & overwrite original file directly" },
        { keys: ["s"], desc: "Open Save dialog (Overwrite / Copy)" }
    ]

    readonly property var editGeneralModel: [
        { keys: ["Esc"], desc: "Cancel active tool / Exit Edit Mode" },
        { keys: ["?"], desc: "Toggle keyboard shortcuts help" }
    ]
}
