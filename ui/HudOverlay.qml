import QtQuick

Rectangle {
    id: root

    required property var theme
    property string mode: "NORMAL" // "NORMAL" or "EDIT"
    property string filename: ""
    property int imgWidth: 0
    property int imgHeight: 0
    property real fileSize: 0
    property real zoomFactor: 1.0
    property int currentIndex: 0
    property int totalCount: 0
    property bool isAnimated: false
    property bool isPlaying: true
    property int currentFrame: 0
    property int frameCount: 1
    property bool autoHide: true
    property bool userActive: true

    implicitHeight: 38
    width: Math.min(parent ? (parent.width - 24) : (contentRow.implicitWidth + 32), contentRow.implicitWidth + 32)
    radius: 19
    color: Qt.rgba(theme.darkerBackground.r, theme.darkerBackground.g, theme.darkerBackground.b, 0.88)
    border.color: Qt.rgba(theme.muted.r, theme.muted.g, theme.muted.b, 0.35)
    border.width: 1

    opacity: (mode === "EDIT" || userActive || hoverHandler.hovered) ? 1.0 : 0.0
    Behavior on opacity { NumberAnimation { duration: 250; easing.type: Easing.OutCubic } }

    HoverHandler {
        id: hoverHandler
    }

    Timer {
        id: hideTimer
        interval: 3500
        repeat: false
        onTriggered: {
            if (root.autoHide && root.mode === "NORMAL" && !hoverHandler.hovered) {
                root.userActive = false;
            }
        }
    }

    function wake() {
        root.userActive = true;
        hideTimer.restart();
    }

    function formatFileSize(bytes) {
        if (!bytes || bytes <= 0) return "";
        if (bytes < 1024) return bytes + " B";
        if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + " KB";
        return (bytes / (1024 * 1024)).toFixed(2) + " MB";
    }

    Row {
        id: contentRow
        anchors.centerIn: parent
        spacing: root.width < 550 ? 8 : 12

        // Mode badge
        Rectangle {
            id: modeBadge
            width: modeText.implicitWidth + 16
            height: 24
            radius: 12
            anchors.verticalCenter: parent.verticalCenter
            color: root.mode === "EDIT" ? theme.accent : theme.surface
            border.color: root.mode === "EDIT" ? theme.accent : Qt.rgba(theme.muted.r, theme.muted.g, theme.muted.b, 0.5)
            border.width: 1

            Text {
                id: modeText
                anchors.centerIn: parent
                text: root.mode
                color: root.mode === "EDIT" ? theme.darkerBackground : theme.foreground
                font.family: theme.fontFamily
                font.pixelSize: 11
                font.bold: true
                font.letterSpacing: 1.0
            }
        }

        // Filename with dynamic width and middle eliding
        Text {
            id: filenameText
            anchors.verticalCenter: parent.verticalCenter
            text: root.filename || "No image"
            color: theme.brightForeground
            font.family: theme.fontFamily
            font.pixelSize: theme.baseFontSize
            font.bold: true
            elide: Text.ElideMiddle
            maximumLineCount: 1
            width: Math.min(implicitWidth, Math.max(60, root.width - (modeBadge.width + (dimText.visible ? dimText.implicitWidth + 14 : 0) + (sizeText.visible ? sizeText.implicitWidth + 14 : 0) + zoomText.implicitWidth + (countBadge.visible ? countBadge.width + 14 : 0) + 90)))
        }

        // Separator dot
        Rectangle {
            width: 3
            height: 3
            radius: 1.5
            anchors.verticalCenter: parent.verticalCenter
            color: theme.muted
            visible: dimText.visible
        }

        // Dimensions
        Text {
            id: dimText
            anchors.verticalCenter: parent.verticalCenter
            text: (root.imgWidth > 0 && root.imgHeight > 0) ? (root.imgWidth + " × " + root.imgHeight) : ""
            color: theme.lightForeground
            font.family: theme.fontFamily
            font.pixelSize: theme.baseFontSize - 1
            visible: text.length > 0 && root.width > 550
        }

        // File size
        Text {
            id: sizeText
            anchors.verticalCenter: parent.verticalCenter
            text: formatFileSize(root.fileSize)
            color: theme.darkForeground
            font.family: theme.fontFamily
            font.pixelSize: theme.baseFontSize - 1
            visible: text.length > 0 && root.width > 620
        }

        // Separator dot
        Rectangle {
            width: 3
            height: 3
            radius: 1.5
            anchors.verticalCenter: parent.verticalCenter
            color: theme.muted
            visible: root.totalCount > 0
        }

        // Zoom percentage
        Text {
            id: zoomText
            anchors.verticalCenter: parent.verticalCenter
            text: Math.round(root.zoomFactor * 100) + "%"
            color: theme.accent
            font.family: theme.fontFamily
            font.pixelSize: theme.baseFontSize - 1
            font.bold: true
        }

        // Animated GIF / WebP playback & frame badge
        Rectangle {
            id: animBadge
            height: 22
            width: animText.implicitWidth + 14
            radius: 11
            anchors.verticalCenter: parent.verticalCenter
            color: root.isPlaying ? Qt.rgba(theme.accent.r, theme.accent.g, theme.accent.b, 0.2) : Qt.rgba(theme.surface.r, theme.surface.g, theme.surface.b, 0.6)
            border.color: root.isPlaying ? theme.accent : Qt.rgba(theme.muted.r, theme.muted.g, theme.muted.b, 0.5)
            border.width: 1
            visible: root.isAnimated && root.frameCount > 1

            Text {
                id: animText
                anchors.centerIn: parent
                text: (root.isPlaying ? "▶ " : "⏸ ") + (root.currentFrame + 1) + "/" + root.frameCount
                color: root.isPlaying ? theme.accent : theme.lightForeground
                font.family: theme.fontFamily
                font.pixelSize: 11
                font.weight: Font.Medium
            }
        }

        // Index / Count badge
        Rectangle {
            id: countBadge
            height: 22
            width: countText.implicitWidth + 14
            radius: 11
            anchors.verticalCenter: parent.verticalCenter
            color: Qt.rgba(theme.surface.r, theme.surface.g, theme.surface.b, 0.6)
            visible: root.totalCount > 0 && root.width > 420

            Text {
                id: countText
                anchors.centerIn: parent
                text: (root.currentIndex + 1) + " / " + root.totalCount
                color: theme.lightForeground
                font.family: theme.fontFamily
                font.pixelSize: 11
                font.weight: Font.Medium
            }
        }
    }
}
