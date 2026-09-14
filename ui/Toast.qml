import QtQuick

Rectangle {
    id: root

    property string message: ""
    property string level: "info" // info, success, warn, error
    property bool shown: false

    implicitWidth: messageText.implicitWidth + 36
    implicitHeight: 40
    radius: 10
    color: theme.darkerBackground
    border.color: {
        if (level === "error") return theme.red;
        if (level === "warn") return theme.yellow;
        if (level === "success") return theme.green;
        return theme.accent;
    }
    border.width: 1.5
    opacity: shown ? 1.0 : 0.0
    scale: shown ? 1.0 : 0.9

    Behavior on opacity { NumberAnimation { duration: 180; easing.type: Easing.OutCubic } }
    Behavior on scale { NumberAnimation { duration: 180; easing.type: Easing.OutBack } }

    Timer {
        id: hideTimer
        interval: 3200
        repeat: false
        onTriggered: root.shown = false
    }

    function show(msg, lvl) {
        root.message = msg;
        root.level = lvl || "info";
        root.shown = true;
        hideTimer.restart();
    }

    Row {
        anchors.centerIn: parent
        spacing: 10

        Rectangle {
            width: 8
            height: 8
            radius: 4
            anchors.verticalCenter: parent.verticalCenter
            color: root.border.color
        }

        Text {
            id: messageText
            text: root.message
            color: theme.brightForeground
            font.family: theme.fontFamily
            font.pixelSize: theme.baseFontSize
            font.weight: Font.DemiBold
            anchors.verticalCenter: parent.verticalCenter
        }
    }
}
