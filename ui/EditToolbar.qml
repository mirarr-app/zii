import QtQuick

Rectangle {
    id: root

    required property var theme
    property bool active: false
    property bool canUndo: false
    property bool canRedo: false
    property bool cropActive: false
    property bool adjustActive: false

    signal triggerCrop()
    signal triggerRotateCW()
    signal triggerRotateCCW()
    signal triggerFlipH()
    signal triggerFlipV()
    signal triggerAdjust()
    signal triggerUndo()
    signal triggerRedo()
    signal triggerSaveOverwrite()
    signal triggerSaveCopy()
    signal triggerExit()

    visible: active
    opacity: active ? 1.0 : 0.0
    scale: active ? 1.0 : 0.95
    Behavior on opacity { NumberAnimation { duration: 200 } }
    Behavior on scale { NumberAnimation { duration: 200; easing.type: Easing.OutBack } }

    implicitHeight: 46
    implicitWidth: buttonRow.implicitWidth + 24
    radius: 23
    color: Qt.rgba(theme.darkerBackground.r, theme.darkerBackground.g, theme.darkerBackground.b, 0.9)
    border.color: Qt.rgba(theme.muted.r, theme.muted.g, theme.muted.b, 0.4)
    border.width: 1

    component ToolBtn: Rectangle {
        id: btn
        property string label: ""
        property string shortcut: ""
        property bool toggled: false
        property bool disabled: false
        signal clicked()

        height: 32
        width: btnRow.implicitWidth + 16
        radius: 16
        color: toggled ? theme.accent : (mouseA.containsMouse ? theme.surface : "transparent")
        border.color: toggled ? theme.accent : (mouseA.containsMouse ? theme.muted : "transparent")
        border.width: 1
        opacity: disabled ? 0.4 : 1.0

        Row {
            id: btnRow
            anchors.centerIn: parent
            spacing: 6

            Text {
                text: btn.label
                color: btn.toggled ? theme.darkerBackground : theme.brightForeground
                font.family: theme.fontFamily
                font.pixelSize: 12
                font.bold: true
                anchors.verticalCenter: parent.verticalCenter
            }

            Rectangle {
                height: 16
                width: scText.implicitWidth + 8
                radius: 4
                color: btn.toggled ? Qt.rgba(0, 0, 0, 0.2) : Qt.rgba(theme.surface.r, theme.surface.g, theme.surface.b, 0.8)
                anchors.verticalCenter: parent.verticalCenter
                visible: btn.shortcut.length > 0

                Text {
                    id: scText
                    anchors.centerIn: parent
                    text: btn.shortcut
                    color: btn.toggled ? theme.darkerBackground : theme.darkForeground
                    font.family: theme.fontFamily
                    font.pixelSize: 9
                    font.bold: true
                }
            }
        }

        MouseArea {
            id: mouseA
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: btn.disabled ? Qt.ArrowCursor : Qt.PointingHandCursor
            enabled: !btn.disabled
            onClicked: btn.clicked()
        }
    }

    Row {
        id: buttonRow
        anchors.centerIn: parent
        spacing: 6

        ToolBtn {
            label: "Crop"
            shortcut: "c"
            toggled: root.cropActive
            onClicked: root.triggerCrop()
        }

        ToolBtn {
            label: "Rotate CW"
            shortcut: "r"
            onClicked: root.triggerRotateCW()
        }

        ToolBtn {
            label: "Rotate CCW"
            shortcut: "R"
            onClicked: root.triggerRotateCCW()
        }

        ToolBtn {
            label: "Flip H"
            shortcut: "h"
            onClicked: root.triggerFlipH()
        }

        ToolBtn {
            label: "Flip V"
            shortcut: "v"
            onClicked: root.triggerFlipV()
        }

        ToolBtn {
            label: "Adjust"
            shortcut: "a"
            toggled: root.adjustActive
            onClicked: root.triggerAdjust()
        }

        // Separator
        Rectangle {
            width: 1
            height: 20
            color: theme.muted
            anchors.verticalCenter: parent.verticalCenter
        }

        ToolBtn {
            label: "Undo"
            shortcut: "u"
            disabled: !root.canUndo
            onClicked: root.triggerUndo()
        }

        ToolBtn {
            label: "Redo"
            shortcut: "^r"
            disabled: !root.canRedo
            onClicked: root.triggerRedo()
        }

        // Separator
        Rectangle {
            width: 1
            height: 20
            color: theme.muted
            anchors.verticalCenter: parent.verticalCenter
        }

        ToolBtn {
            label: "Save"
            shortcut: "w"
            onClicked: root.triggerSaveOverwrite()
        }

        ToolBtn {
            label: "Save Copy"
            shortcut: "s"
            onClicked: root.triggerSaveCopy()
        }

        ToolBtn {
            label: "Exit"
            shortcut: "Esc"
            onClicked: root.triggerExit()
        }
    }
}
