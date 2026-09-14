import QtQuick

Rectangle {
    id: root

    required property var theme
    property bool active: false
    property int brightness: 0
    property real contrast: 0.0 // -100 to 100

    signal adjustmentsApplied(int brightness, real contrast)
    signal closed()

    visible: active
    opacity: active ? 1.0 : 0.0
    scale: active ? 1.0 : 0.95
    Behavior on opacity { NumberAnimation { duration: 180 } }
    Behavior on scale { NumberAnimation { duration: 180; easing.type: Easing.OutBack } }

    width: 320
    height: 190
    radius: 12
    color: Qt.rgba(theme.darkerBackground.r, theme.darkerBackground.g, theme.darkerBackground.b, 0.92)
    border.color: theme.accent
    border.width: 1

    Column {
        anchors.fill: parent
        anchors.margins: 18
        spacing: 14

        Item {
            width: parent.width
            height: 20
            Text {
                anchors.left: parent.left
                anchors.verticalCenter: parent.verticalCenter
                text: "Image Adjustments"
                color: theme.brightForeground
                font.family: theme.fontFamily
                font.pixelSize: theme.baseFontSize
                font.bold: true
            }
            Text {
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                text: "Esc to close"
                color: theme.darkForeground
                font.family: theme.fontFamily
                font.pixelSize: 11
            }
        }

        // Brightness Slider
        Column {
            width: parent.width
            spacing: 4

            Item {
                width: parent.width
                height: 16
                Text {
                    anchors.left: parent.left
                    anchors.verticalCenter: parent.verticalCenter
                    text: "Brightness"
                    color: theme.lightForeground
                    font.family: theme.fontFamily
                    font.pixelSize: 11
                }
                Text {
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    text: (root.brightness > 0 ? "+" : "") + root.brightness
                    color: theme.accent
                    font.family: theme.fontFamily
                    font.pixelSize: 11
                    font.bold: true
                }
            }

            Rectangle {
                id: brightTrack
                width: parent.width
                height: 6
                radius: 3
                color: theme.selection

                Rectangle {
                    width: Math.max(0, (root.brightness + 100) / 200 * parent.width)
                    height: parent.height
                    radius: 3
                    color: theme.accent
                }

                Rectangle {
                    id: brightThumb
                    x: Math.max(0, Math.min(parent.width - 14, (root.brightness + 100) / 200 * parent.width - 7))
                    y: -4
                    width: 14
                    height: 14
                    radius: 7
                    color: theme.brightForeground
                    border.color: theme.accent
                    border.width: 2
                }

                MouseArea {
                    anchors.fill: parent
                    anchors.margins: -8
                    cursorShape: Qt.PointingHandCursor
                    onPositionChanged: mouse => {
                        var frac = Math.max(0, Math.min(1, mouse.x / parent.width));
                        root.brightness = Math.round((frac * 200) - 100);
                        applyDebounce.restart();
                    }
                    onClicked: mouse => {
                        var frac = Math.max(0, Math.min(1, mouse.x / parent.width));
                        root.brightness = Math.round((frac * 200) - 100);
                        applyDebounce.restart();
                    }
                }
            }
        }

        // Contrast Slider
        Column {
            width: parent.width
            spacing: 4

            Item {
                width: parent.width
                height: 16
                Text {
                    anchors.left: parent.left
                    anchors.verticalCenter: parent.verticalCenter
                    text: "Contrast"
                    color: theme.lightForeground
                    font.family: theme.fontFamily
                    font.pixelSize: 11
                }
                Text {
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    text: (root.contrast > 0 ? "+" : "") + Math.round(root.contrast)
                    color: theme.accent
                    font.family: theme.fontFamily
                    font.pixelSize: 11
                    font.bold: true
                }
            }

            Rectangle {
                id: contrastTrack
                width: parent.width
                height: 6
                radius: 3
                color: theme.selection

                Rectangle {
                    width: Math.max(0, (root.contrast + 100) / 200 * parent.width)
                    height: parent.height
                    radius: 3
                    color: theme.accent
                }

                Rectangle {
                    id: contrastThumb
                    x: Math.max(0, Math.min(parent.width - 14, (root.contrast + 100) / 200 * parent.width - 7))
                    y: -4
                    width: 14
                    height: 14
                    radius: 7
                    color: theme.brightForeground
                    border.color: theme.accent
                    border.width: 2
                }

                MouseArea {
                    anchors.fill: parent
                    anchors.margins: -8
                    cursorShape: Qt.PointingHandCursor
                    onPositionChanged: mouse => {
                        var frac = Math.max(0, Math.min(1, mouse.x / parent.width));
                        root.contrast = Math.round((frac * 200) - 100);
                        applyDebounce.restart();
                    }
                    onClicked: mouse => {
                        var frac = Math.max(0, Math.min(1, mouse.x / parent.width));
                        root.contrast = Math.round((frac * 200) - 100);
                        applyDebounce.restart();
                    }
                }
            }
        }

        // Reset Button
        Row {
            anchors.horizontalCenter: parent.horizontalCenter
            spacing: 12

            Rectangle {
                width: 90
                height: 26
                radius: 6
                color: theme.surface
                border.color: theme.muted
                border.width: 1
                Text {
                    anchors.centerIn: parent
                    text: "Reset"
                    color: theme.foreground
                    font.family: theme.fontFamily
                    font.pixelSize: 11
                }
                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    onClicked: {
                        root.brightness = 0;
                        root.contrast = 0;
                        applyDebounce.restart();
                    }
                }
            }
        }
    }

    Timer {
        id: applyDebounce
        interval: 120
        repeat: false
        onTriggered: {
            root.adjustmentsApplied(root.brightness, root.contrast);
        }
    }

    function stepBrightness(delta) {
        root.brightness = Math.max(-100, Math.min(100, root.brightness + delta));
        applyDebounce.restart();
    }

    function stepContrast(delta) {
        root.contrast = Math.max(-100, Math.min(100, root.contrast + delta));
        applyDebounce.restart();
    }
}
