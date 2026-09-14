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
    property string aspectRatio: "FREE" // "FREE", "1:1", "16:9", "4:3", "3:2", "9:16"

    signal cropApplied(real x, real y, real width, real height)
    signal cropCanceled()

    visible: active
    enabled: active

    function getRatioValue(ratio) {
        if (ratio === "1:1") return 1.0;
        if (ratio === "16:9") return 16.0 / 9.0;
        if (ratio === "4:3") return 4.0 / 3.0;
        if (ratio === "3:2") return 3.0 / 2.0;
        if (ratio === "9:16") return 9.0 / 16.0;
        return 0.0;
    }

    function setAspectRatio(newRatio) {
        aspectRatio = newRatio;
        if (newRatio === "FREE") return;
        var r = getRatioValue(newRatio);
        if (r <= 0) return;

        // Centers and fits the crop rectangle to the image with that ratio
        var maxW = imgNaturalWidth * 0.9;
        var maxH = imgNaturalHeight * 0.9;
        var w = maxW;
        var h = w / r;
        if (h > maxH) {
            h = maxH;
            w = h * r;
        }
        cropW = Math.round(w);
        cropH = Math.round(h);
        cropX = Math.round((imgNaturalWidth - cropW) / 2);
        cropY = Math.round((imgNaturalHeight - cropH) / 2);
        clampCrop();
    }

    function initCrop() {
        if (aspectRatio === "FREE") {
            cropX = Math.round(imgNaturalWidth * 0.1);
            cropY = Math.round(imgNaturalHeight * 0.1);
            cropW = Math.round(imgNaturalWidth * 0.8);
            cropH = Math.round(imgNaturalHeight * 0.8);
        } else {
            setAspectRatio(aspectRatio);
        }
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

    // Clamp helper to ensure crop bounds stay within the image and enforce aspect ratio
    function clampCrop() {
        if (aspectRatio === "FREE") {
            cropW = Math.max(10, Math.min(imgNaturalWidth, cropW));
            cropH = Math.max(10, Math.min(imgNaturalHeight, cropH));
            cropX = Math.max(0, Math.min(imgNaturalWidth - cropW, cropX));
            cropY = Math.max(0, Math.min(imgNaturalHeight - cropH, cropY));
        } else {
            var r = getRatioValue(aspectRatio);
            if (r > 0) {
                var maxW = imgNaturalWidth;
                var maxH = maxW / r;
                if (maxH > imgNaturalHeight) {
                    maxH = imgNaturalHeight;
                    maxW = maxH * r;
                }
                cropW = Math.max(20, Math.min(maxW, cropW));
                cropH = cropW / r;
                cropX = Math.max(0, Math.min(imgNaturalWidth - cropW, cropX));
                cropY = Math.max(0, Math.min(imgNaturalHeight - cropH, cropY));
            }
        }
    }

    // Keyboard controls for crop - scaled by zoomFactor to ensure consistent screen motion
    function moveBox(dirX, dirY) {
        var step = Math.max(5, Math.round(20 / Math.max(0.001, root.zoomFactor)));
        cropX += dirX * step;
        cropY += dirY * step;
        clampCrop();
    }

    function resizeBox(dirW, dirH) {
        var step = Math.max(5, Math.round(25 / Math.max(0.001, root.zoomFactor)));
        if (aspectRatio !== "FREE") {
            var r = getRatioValue(aspectRatio);
            if (r > 0) {
                var delta = 0;
                if (dirW !== 0) delta = dirW * step;
                else if (dirH !== 0) delta = dirH * step * r;
                var newW = cropW + delta;
                var newH = newW / r;
                if (newW >= 20 && newH >= 20) {
                    var diffW = newW - cropW;
                    var diffH = newH - cropH;
                    var newX = cropX - diffW / 2;
                    var newY = cropY - diffH / 2;
                    cropX = Math.max(0, Math.min(imgNaturalWidth - newW, newX));
                    cropY = Math.max(0, Math.min(imgNaturalHeight - newH, newY));
                    cropW = newW;
                    cropH = newH;
                }
                clampCrop();
                return;
            }
        }
        if (dirW > 0) cropW += step;
        else if (dirW < 0) cropW = Math.max(20, cropW - step);
        if (dirH > 0) cropH += step;
        else if (dirH < 0) cropH = Math.max(20, cropH - step);
        clampCrop();
    }

    function adjustEdge(edge, dir) {
        var step = Math.max(5, Math.round(25 / Math.max(0.001, root.zoomFactor)));
        if (aspectRatio !== "FREE") {
            var r = getRatioValue(aspectRatio);
            if (r > 0) {
                var delta = (edge === "left" ? -dir : -dir) * step;
                var newW = cropW + delta;
                var newH = newW / r;
                if (newW >= 20 && newH >= 20) {
                    if (edge === "left") {
                        var dW = newW - cropW;
                        cropX = Math.max(0, cropX - dW);
                    } else if (edge === "top") {
                        var dH = newH - cropH;
                        cropY = Math.max(0, cropY - dH);
                    }
                    cropW = newW;
                    cropH = newH;
                }
                clampCrop();
                return;
            }
        }
        if (edge === "left") {
            if (dir < 0) {
                var actualL = Math.min(cropX, step);
                cropX -= actualL;
                cropW += actualL;
            } else {
                var actualShrinkL = Math.min(cropW - 20, step);
                cropX += actualShrinkL;
                cropW -= actualShrinkL;
            }
        } else if (edge === "top") {
            if (dir < 0) {
                var actualT = Math.min(cropY, step);
                cropY -= actualT;
                cropH += actualT;
            } else {
                var actualShrinkT = Math.min(cropH - 20, step);
                cropY += actualShrinkT;
                cropH -= actualShrinkT;
            }
        }
        clampCrop();
    }

    function moveLeft(step) { moveBox(-1, 0); }
    function moveRight(step) { moveBox(1, 0); }
    function moveUp(step) { moveBox(0, -1); }
    function moveDown(step) { moveBox(0, 1); }

    function shrinkW(step) { resizeBox(-1, 0); }
    function expandW(step) { resizeBox(1, 0); }
    function shrinkH(step) { resizeBox(0, -1); }
    function expandH(step) { resizeBox(0, 1); }

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
                    if (root.aspectRatio !== "FREE") {
                        var r = root.getRatioValue(root.aspectRatio);
                        var fixedRight = sCropX + sCropW;
                        var fixedBottom = sCropY + sCropH;
                        var newW = Math.max(20, sCropW - dx);
                        var newH = newW / r;
                        root.cropX = fixedRight - newW;
                        root.cropY = fixedBottom - newH;
                        root.cropW = newW;
                        root.cropH = newH;
                    } else {
                        root.cropX = sCropX + dx;
                        root.cropY = sCropY + dy;
                        root.cropW = sCropW - dx;
                        root.cropH = sCropH - dy;
                    }
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
                property real startY: 0; property real sCropX: 0; property real sCropY: 0; property real sCropW: 0; property real sCropH: 0
                onPressed: m => { startY = m.y; sCropX = root.cropX; sCropY = root.cropY; sCropW = root.cropW; sCropH = root.cropH; }
                onPositionChanged: m => {
                    var dy = (m.y - startY) / root.zoomFactor;
                    if (root.aspectRatio !== "FREE") {
                        var r = root.getRatioValue(root.aspectRatio);
                        var fixedBottom = sCropY + sCropH;
                        var newH = Math.max(20, sCropH - dy);
                        var newW = newH * r;
                        var diffW = newW - sCropW;
                        root.cropX = sCropX - diffW / 2;
                        root.cropY = fixedBottom - newH;
                        root.cropW = newW;
                        root.cropH = newH;
                    } else {
                        root.cropY = sCropY + dy;
                        root.cropH = sCropH - dy;
                    }
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
                property real sCropX: 0; property real sCropY: 0; property real sCropW: 0; property real sCropH: 0
                onPressed: m => { startX = m.x; startY = m.y; sCropX = root.cropX; sCropY = root.cropY; sCropW = root.cropW; sCropH = root.cropH; }
                onPositionChanged: m => {
                    var dx = (m.x - startX) / root.zoomFactor;
                    var dy = (m.y - startY) / root.zoomFactor;
                    if (root.aspectRatio !== "FREE") {
                        var r = root.getRatioValue(root.aspectRatio);
                        var fixedBottom = sCropY + sCropH;
                        var newW = Math.max(20, sCropW + dx);
                        var newH = newW / r;
                        root.cropX = sCropX;
                        root.cropY = fixedBottom - newH;
                        root.cropW = newW;
                        root.cropH = newH;
                    } else {
                        root.cropY = sCropY + dy;
                        root.cropW = sCropW + dx;
                        root.cropH = sCropH - dy;
                    }
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
                property real startX: 0
                property real sCropX: 0; property real sCropY: 0; property real sCropW: 0; property real sCropH: 0
                onPressed: m => { startX = m.x; sCropX = root.cropX; sCropY = root.cropY; sCropW = root.cropW; sCropH = root.cropH; }
                onPositionChanged: m => {
                    var dx = (m.x - startX) / root.zoomFactor;
                    if (root.aspectRatio !== "FREE") {
                        var r = root.getRatioValue(root.aspectRatio);
                        var newW = Math.max(20, sCropW + dx);
                        var newH = newW / r;
                        var diffH = newH - sCropH;
                        root.cropX = sCropX;
                        root.cropY = sCropY - diffH / 2;
                        root.cropW = newW;
                        root.cropH = newH;
                    } else {
                        root.cropW = sCropW + dx;
                    }
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
                property real sCropX: 0; property real sCropY: 0; property real sCropW: 0; property real sCropH: 0
                onPressed: m => { startX = m.x; startY = m.y; sCropX = root.cropX; sCropY = root.cropY; sCropW = root.cropW; sCropH = root.cropH; }
                onPositionChanged: m => {
                    var dx = (m.x - startX) / root.zoomFactor;
                    var dy = (m.y - startY) / root.zoomFactor;
                    if (root.aspectRatio !== "FREE") {
                        var r = root.getRatioValue(root.aspectRatio);
                        var newW = Math.max(20, sCropW + dx);
                        var newH = newW / r;
                        root.cropX = sCropX;
                        root.cropY = sCropY;
                        root.cropW = newW;
                        root.cropH = newH;
                    } else {
                        root.cropW = sCropW + dx;
                        root.cropH = sCropH + dy;
                    }
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
                property real startY: 0; property real sCropX: 0; property real sCropY: 0; property real sCropW: 0; property real sCropH: 0
                onPressed: m => { startY = m.y; sCropX = root.cropX; sCropY = root.cropY; sCropW = root.cropW; sCropH = root.cropH; }
                onPositionChanged: m => {
                    var dy = (m.y - startY) / root.zoomFactor;
                    if (root.aspectRatio !== "FREE") {
                        var r = root.getRatioValue(root.aspectRatio);
                        var newH = Math.max(20, sCropH + dy);
                        var newW = newH * r;
                        var diffW = newW - sCropW;
                        root.cropX = sCropX - diffW / 2;
                        root.cropY = sCropY;
                        root.cropW = newW;
                        root.cropH = newH;
                    } else {
                        root.cropH = sCropH + dy;
                    }
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
                property real sCropX: 0; property real sCropY: 0; property real sCropW: 0; property real sCropH: 0
                onPressed: m => { startX = m.x; startY = m.y; sCropX = root.cropX; sCropY = root.cropY; sCropW = root.cropW; sCropH = root.cropH; }
                onPositionChanged: m => {
                    var dx = (m.x - startX) / root.zoomFactor;
                    var dy = (m.y - startY) / root.zoomFactor;
                    if (root.aspectRatio !== "FREE") {
                        var r = root.getRatioValue(root.aspectRatio);
                        var fixedRight = sCropX + sCropW;
                        var newW = Math.max(20, sCropW - dx);
                        var newH = newW / r;
                        root.cropX = fixedRight - newW;
                        root.cropY = sCropY;
                        root.cropW = newW;
                        root.cropH = newH;
                    } else {
                        root.cropX = sCropX + dx;
                        root.cropW = sCropW - dx;
                        root.cropH = sCropH + dy;
                    }
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
                property real startX: 0
                property real sCropX: 0; property real sCropY: 0; property real sCropW: 0; property real sCropH: 0
                onPressed: m => { startX = m.x; sCropX = root.cropX; sCropY = root.cropY; sCropW = root.cropW; sCropH = root.cropH; }
                onPositionChanged: m => {
                    var dx = (m.x - startX) / root.zoomFactor;
                    if (root.aspectRatio !== "FREE") {
                        var r = root.getRatioValue(root.aspectRatio);
                        var fixedRight = sCropX + sCropW;
                        var newW = Math.max(20, sCropW - dx);
                        var newH = newW / r;
                        var diffH = newH - sCropH;
                        root.cropX = fixedRight - newW;
                        root.cropY = sCropY - diffH / 2;
                        root.cropW = newW;
                        root.cropH = newH;
                    } else {
                        root.cropX = sCropX + dx;
                        root.cropW = sCropW - dx;
                    }
                    root.clampCrop();
                }
            }
        }
    }

    // Sleek Aspect Ratio Selector Bar
    Rectangle {
        id: ratioBar
        anchors.top: parent.top
        anchors.topMargin: 72
        anchors.horizontalCenter: parent.horizontalCenter
        height: 36
        width: ratioRow.implicitWidth + 20
        radius: 18
        color: Qt.rgba(theme.darkerBackground.r, theme.darkerBackground.g, theme.darkerBackground.b, 0.94)
        border.color: theme.accent
        border.width: 1
        z: 100

        Row {
            id: ratioRow
            anchors.centerIn: parent
            spacing: 4

            Repeater {
                model: [
                    { id: "FREE", label: "Free" },
                    { id: "1:1", label: "1:1" },
                    { id: "16:9", label: "16:9" },
                    { id: "4:3", label: "4:3" },
                    { id: "3:2", label: "3:2" },
                    { id: "9:16", label: "9:16" }
                ]

                delegate: Rectangle {
                    id: ratioBtn
                    height: 26
                    width: btnText.implicitWidth + 16
                    radius: 13
                    property bool isSelected: root.aspectRatio === modelData.id
                    color: isSelected ? theme.accent : (btnMouse.containsMouse ? theme.selection : "transparent")

                    Text {
                        id: btnText
                        anchors.centerIn: parent
                        text: modelData.label
                        color: ratioBtn.isSelected ? theme.darkerBackground : (btnMouse.containsMouse ? theme.brightForeground : theme.lightForeground)
                        font.family: theme.fontFamily
                        font.pixelSize: 11
                        font.bold: ratioBtn.isSelected
                    }

                    MouseArea {
                        id: btnMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            root.setAspectRatio(modelData.id);
                        }
                    }
                }
            }
        }
    }
}
