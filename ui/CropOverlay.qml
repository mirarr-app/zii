import QtQuick

Item {
    id: root

    required property var theme
    property real imgNaturalWidth: 100
    property real imgNaturalHeight: 100
    property real panX: 0
    property real panY: 0
    property real zoomFactor: 1.0
    property bool active: false

    // Crop box coordinates relative to the original image in pixels
    property real cropX: 0
    property real cropY: 0
    property real cropW: 100
    property real cropH: 100

    signal cropApplied(real x, real y, real width, real height)
    signal cropCanceled()

    visible: active
    enabled: active

    function initCrop() {
        cropX = Math.round(imgNaturalWidth * 0.1);
        cropY = Math.round(imgNaturalHeight * 0.1);
        cropW = Math.round(imgNaturalWidth * 0.8);
        cropH = Math.round(imgNaturalHeight * 0.8);
    }

    onActiveChanged: {
        if (active) {
            initCrop();
        }
    }

    // Screen coordinates of the crop rectangle
    readonly property real screenX: panX + cropX * zoomFactor
    readonly property real screenY: panY + cropY * zoomFactor
    readonly property real screenW: cropW * zoomFactor
    readonly property real screenH: cropH * zoomFactor

    // Clamp helper to ensure crop bounds stay within the image
    function clampCrop() {
        cropW = Math.max(10, Math.min(imgNaturalWidth, cropW));
        cropH = Math.max(10, Math.min(imgNaturalHeight, cropH));
        cropX = Math.max(0, Math.min(imgNaturalWidth - cropW, cropX));
        cropY = Math.max(0, Math.min(imgNaturalHeight - cropH, cropY));
    }

    // Keyboard controls for crop
    function moveLeft(step) { cropX -= (step || 10); clampCrop(); }
    function moveRight(step) { cropX += (step || 10); clampCrop(); }
    function moveUp(step) { cropY -= (step || 10); clampCrop(); }
    function moveDown(step) { cropY += (step || 10); clampCrop(); }

    function shrinkW(step) { cropW -= (step || 10); clampCrop(); }
    function expandW(step) { cropW += (step || 10); clampCrop(); }
    function shrinkH(step) { cropH -= (step || 10); clampCrop(); }
    function expandH(step) { cropH += (step || 10); clampCrop(); }

    function apply() {
        clampCrop();
        cropApplied(Math.round(cropX), Math.round(cropY), Math.round(cropW), Math.round(cropH));
    }

    function cancel() {
        cropCanceled();
    }

    // Dark scrim around the crop area
    // Top
    Rectangle {
        x: 0; y: 0; width: root.width; height: Math.max(0, root.screenY)
        color: Qt.rgba(0, 0, 0, 0.65)
    }
    // Bottom
    Rectangle {
        x: 0; y: root.screenY + root.screenH; width: root.width; height: Math.max(0, root.height - (root.screenY + root.screenH))
        color: Qt.rgba(0, 0, 0, 0.65)
    }
    // Left
    Rectangle {
        x: 0; y: root.screenY; width: Math.max(0, root.screenX); height: root.screenH
        color: Qt.rgba(0, 0, 0, 0.65)
    }
    // Right
    Rectangle {
        x: root.screenX + root.screenW; y: root.screenY; width: Math.max(0, root.width - (root.screenX + root.screenW)); height: root.screenH
        color: Qt.rgba(0, 0, 0, 0.65)
    }

    // Crop box container
    Item {
        x: root.screenX
        y: root.screenY
        width: root.screenW
        height: root.screenH

        // Border outline
        Rectangle {
            anchors.fill: parent
            color: "transparent"
            border.color: theme.accent
            border.width: 2
        }

        // Rule of thirds grid lines
        // Horizontal 1/3
        Rectangle {
            x: 0; y: parent.height / 3; width: parent.width; height: 1
            color: Qt.rgba(255, 255, 255, 0.3)
        }
        // Horizontal 2/3
        Rectangle {
            x: 0; y: parent.height * 2 / 3; width: parent.width; height: 1
            color: Qt.rgba(255, 255, 255, 0.3)
        }
        // Vertical 1/3
        Rectangle {
            x: parent.width / 3; y: 0; width: 1; height: parent.height
            color: Qt.rgba(255, 255, 255, 0.3)
        }
        // Vertical 2/3
        Rectangle {
            x: parent.width * 2 / 3; y: 0; width: 1; height: parent.height
            color: Qt.rgba(255, 255, 255, 0.3)
        }

        // Central drag area to move the entire crop box
        MouseArea {
            anchors.fill: parent
            anchors.margins: 12
            cursorShape: Qt.SizeAllCursor
            property real startMouseX: 0
            property real startMouseY: 0
            property real startCropX: 0
            property real startCropY: 0

            onPressed: mouse => {
                startMouseX = mouse.x;
                startMouseY = mouse.y;
                startCropX = root.cropX;
                startCropY = root.cropY;
            }

            onPositionChanged: mouse => {
                var dx = (mouse.x - startMouseX) / root.zoomFactor;
                var dy = (mouse.y - startMouseY) / root.zoomFactor;
                root.cropX = startCropX + dx;
                root.cropY = startCropY + dy;
                root.clampCrop();
            }
        }

        // Dimension indicator pill
        Rectangle {
            anchors.bottom: parent.top
            anchors.bottomMargin: 8
            anchors.horizontalCenter: parent.horizontalCenter
            height: 24
            width: dimText.implicitWidth + 16
            radius: 12
            color: Qt.rgba(theme.darkerBackground.r, theme.darkerBackground.g, theme.darkerBackground.b, 0.9)
            border.color: theme.accent
            border.width: 1

            Text {
                id: dimText
                anchors.centerIn: parent
                text: Math.round(root.cropW) + " × " + Math.round(root.cropH)
                color: theme.brightForeground
                font.family: theme.fontFamily
                font.pixelSize: 11
                font.bold: true
            }
        }

        // Action buttons above crop box
        Row {
            anchors.top: parent.bottom
            anchors.topMargin: 10
            anchors.horizontalCenter: parent.horizontalCenter
            spacing: 8

            Rectangle {
                height: 28
                width: applyText.implicitWidth + 20
                radius: 6
                color: theme.accent
                Text {
                    id: applyText
                    anchors.centerIn: parent
                    text: "Apply (Enter)"
                    color: theme.darkerBackground
                    font.family: theme.fontFamily
                    font.pixelSize: 11
                    font.bold: true
                }
                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    onClicked: root.apply()
                }
            }

            Rectangle {
                height: 28
                width: cancelText.implicitWidth + 20
                radius: 6
                color: theme.surface
                border.color: theme.muted
                border.width: 1
                Text {
                    id: cancelText
                    anchors.centerIn: parent
                    text: "Cancel (Esc)"
                    color: theme.foreground
                    font.family: theme.fontFamily
                    font.pixelSize: 11
                }
                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    onClicked: root.cancel()
                }
            }
        }

        // 8 Resize Handles
        // Helper component for handle dots
        component Handle: Rectangle {
            width: 14
            height: 14
            radius: 7
            color: theme.brightForeground
            border.color: theme.accent
            border.width: 2
        }

        // Top-Left
        Handle {
            anchors.horizontalCenter: parent.left
            anchors.verticalCenter: parent.top
            MouseArea {
                anchors.fill: parent; anchors.margins: -8; cursorShape: Qt.SizeFDiagCursor
                property real startX: 0; property real startY: 0
                property real sCropX: 0; property real sCropY: 0; property real sCropW: 0; property real sCropH: 0
                onPressed: m => { startX = m.x; startY = m.y; sCropX = root.cropX; sCropY = root.cropY; sCropW = root.cropW; sCropH = root.cropH; }
                onPositionChanged: m => {
                    var dx = (m.x - startX) / root.zoomFactor;
                    var dy = (m.y - startY) / root.zoomFactor;
                    root.cropX = sCropX + dx;
                    root.cropY = sCropY + dy;
                    root.cropW = sCropW - dx;
                    root.cropH = sCropH - dy;
                    root.clampCrop();
                }
            }
        }

        // Top
        Handle {
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.verticalCenter: parent.top
            MouseArea {
                anchors.fill: parent; anchors.margins: -8; cursorShape: Qt.SizeVerCursor
                property real startY: 0; property real sCropY: 0; property real sCropH: 0
                onPressed: m => { startY = m.y; sCropY = root.cropY; sCropH = root.cropH; }
                onPositionChanged: m => {
                    var dy = (m.y - startY) / root.zoomFactor;
                    root.cropY = sCropY + dy;
                    root.cropH = sCropH - dy;
                    root.clampCrop();
                }
            }
        }

        // Top-Right
        Handle {
            anchors.horizontalCenter: parent.right
            anchors.verticalCenter: parent.top
            MouseArea {
                anchors.fill: parent; anchors.margins: -8; cursorShape: Qt.SizeBDiagCursor
                property real startX: 0; property real startY: 0
                property real sCropY: 0; property real sCropW: 0; property real sCropH: 0
                onPressed: m => { startX = m.x; startY = m.y; sCropY = root.cropY; sCropW = root.cropW; sCropH = root.cropH; }
                onPositionChanged: m => {
                    var dx = (m.x - startX) / root.zoomFactor;
                    var dy = (m.y - startY) / root.zoomFactor;
                    root.cropY = sCropY + dy;
                    root.cropW = sCropW + dx;
                    root.cropH = sCropH - dy;
                    root.clampCrop();
                }
            }
        }

        // Right
        Handle {
            anchors.horizontalCenter: parent.right
            anchors.verticalCenter: parent.verticalCenter
            MouseArea {
                anchors.fill: parent; anchors.margins: -8; cursorShape: Qt.SizeHorCursor
                property real startX: 0; property real sCropW: 0
                onPressed: m => { startX = m.x; sCropW = root.cropW; }
                onPositionChanged: m => {
                    var dx = (m.x - startX) / root.zoomFactor;
                    root.cropW = sCropW + dx;
                    root.clampCrop();
                }
            }
        }

        // Bottom-Right
        Handle {
            anchors.horizontalCenter: parent.right
            anchors.verticalCenter: parent.bottom
            MouseArea {
                anchors.fill: parent; anchors.margins: -8; cursorShape: Qt.SizeFDiagCursor
                property real startX: 0; property real startY: 0
                property real sCropW: 0; property real sCropH: 0
                onPressed: m => { startX = m.x; startY = m.y; sCropW = root.cropW; sCropH = root.cropH; }
                onPositionChanged: m => {
                    var dx = (m.x - startX) / root.zoomFactor;
                    var dy = (m.y - startY) / root.zoomFactor;
                    root.cropW = sCropW + dx;
                    root.cropH = sCropH + dy;
                    root.clampCrop();
                }
            }
        }

        // Bottom
        Handle {
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.verticalCenter: parent.bottom
            MouseArea {
                anchors.fill: parent; anchors.margins: -8; cursorShape: Qt.SizeVerCursor
                property real startY: 0; property real sCropH: 0
                onPressed: m => { startY = m.y; sCropH = root.cropH; }
                onPositionChanged: m => {
                    var dy = (m.y - startY) / root.zoomFactor;
                    root.cropH = sCropH + dy;
                    root.clampCrop();
                }
            }
        }

        // Bottom-Left
        Handle {
            anchors.horizontalCenter: parent.left
            anchors.verticalCenter: parent.bottom
            MouseArea {
                anchors.fill: parent; anchors.margins: -8; cursorShape: Qt.SizeBDiagCursor
                property real startX: 0; property real startY: 0
                property real sCropX: 0; property real sCropW: 0; property real sCropH: 0
                onPressed: m => { startX = m.x; startY = m.y; sCropX = root.cropX; sCropW = root.cropW; sCropH = root.cropH; }
                onPositionChanged: m => {
                    var dx = (m.x - startX) / root.zoomFactor;
                    var dy = (m.y - startY) / root.zoomFactor;
                    root.cropX = sCropX + dx;
                    root.cropW = sCropW - dx;
                    root.cropH = sCropH + dy;
                    root.clampCrop();
                }
            }
        }

        // Left
        Handle {
            anchors.horizontalCenter: parent.left
            anchors.verticalCenter: parent.verticalCenter
            MouseArea {
                anchors.fill: parent; anchors.margins: -8; cursorShape: Qt.SizeHorCursor
                property real startX: 0; property real sCropX: 0; property real sCropW: 0
                onPressed: m => { startX = m.x; sCropX = root.cropX; sCropW = root.cropW; }
                onPositionChanged: m => {
                    var dx = (m.x - startX) / root.zoomFactor;
                    root.cropX = sCropX + dx;
                    root.cropW = sCropW - dx;
                    root.clampCrop();
                }
            }
        }
    }
}
