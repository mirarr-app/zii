import QtQuick

Item {
    id: root

    required property var theme
    property string source: ""
    property real imgNaturalWidth: 0
    property real imgNaturalHeight: 0
    property real zoomFactor: 1.0
    property real panX: 0
    property real panY: 0
    property bool isEditing: false

    signal interactionOccurred()
    signal doubleClicked()

    property bool userInteracted: false

    // Animated format detection (.gif or .webp)
    readonly property bool isAnimated: {
        if (!source) return false;
        var s = source.toLowerCase();
        return s.indexOf(".gif") !== -1 || s.indexOf(".webp") !== -1;
    }

    // Playback status and frame indicators exposed
    readonly property bool isPlaying: isAnimated && (frameCount > 1) && (!animImage.paused && animImage.playing)
    readonly property int currentFrame: isAnimated ? animImage.currentFrame : 0
    readonly property int frameCount: isAnimated ? animImage.frameCount : 1

    clip: true

    onSourceChanged: {
        if (!isEditing) {
            userInteracted = false;
            if (animImage) animImage.paused = false;
            resetView();
        }
    }

    onWidthChanged: {
        if (!isEditing && !userInteracted && width > 0 && height > 0) {
            resetView();
        }
    }

    onHeightChanged: {
        if (!isEditing && !userInteracted && width > 0 && height > 0) {
            resetView();
        }
    }

    function togglePlayback() {
        if (isAnimated && frameCount > 1) {
            animImage.paused = !animImage.paused;
            interactionOccurred();
        }
    }

    function activeSourceWidth() {
        var item = isAnimated ? animImage : mainImage;
        return (item && item.sourceSize.width > 0) ? item.sourceSize.width : 0;
    }

    function activeSourceHeight() {
        var item = isAnimated ? animImage : mainImage;
        return (item && item.sourceSize.height > 0) ? item.sourceSize.height : 0;
    }

    function naturalW() {
        var sw = activeSourceWidth();
        return imgNaturalWidth > 0 ? imgNaturalWidth : (sw > 0 ? sw : 0);
    }

    function naturalH() {
        var sh = activeSourceHeight();
        return imgNaturalHeight > 0 ? imgNaturalHeight : (sh > 0 ? sh : 0);
    }

    function clampPanX(val, curW) {
        if (width <= 0 || curW <= 0) return val;
        // Keep at least 50px or 20% of image width inside the viewport
        var margin = Math.min(50, curW * 0.2);
        var minX = margin - curW;
        var maxX = width - margin;
        if (minX > maxX) return Math.round((width - curW) / 2);
        return Math.max(minX, Math.min(maxX, val));
    }

    function clampPanY(val, curH) {
        if (height <= 0 || curH <= 0) return val;
        // Keep at least 50px or 20% of image height inside the viewport
        var margin = Math.min(50, curH * 0.2);
        var minY = margin - curH;
        var maxY = height - margin;
        if (minY > maxY) return Math.round((height - curH) / 2);
        return Math.max(minY, Math.min(maxY, val));
    }

    function fitScale() {
        var nw = naturalW();
        var nh = naturalH();

        if (nw <= 0 || nh <= 0 || width <= 0 || height <= 0) {
            return 1.0;
        }
        var scaleX = width / nw;
        var scaleY = height / nh;
        return Math.min(scaleX, scaleY, 1.0);
    }

    function resetView() {
        var nw = naturalW();
        var nh = naturalH();

        if (nw <= 0 || nh <= 0 || width <= 0 || height <= 0) {
            zoomFactor = 1.0;
            panX = 0;
            panY = 0;
            return;
        }

        var fit = Math.min(width / nw, height / nh, 1.0);
        zoomFactor = fit;
        panX = Math.round((width - nw * fit) / 2);
        panY = Math.round((height - nh * fit) / 2);
    }

    function setZoom100() {
        zoomTo(1.0, width / 2, height / 2);
    }

    function zoomIn() {
        zoomTo(zoomFactor * 1.25, width / 2, height / 2);
    }

    function zoomOut() {
        zoomTo(zoomFactor * 0.8, width / 2, height / 2);
    }

    function zoomTo(newScale, focalX, focalY) {
        newScale = Math.max(0.05, Math.min(30.0, newScale));
        if (Math.abs(newScale - zoomFactor) < 0.0001) return;

        var imgX = (focalX - panX) / zoomFactor;
        var imgY = (focalY - panY) / zoomFactor;

        userInteracted = true;
        zoomFactor = newScale;

        var targetPanX = focalX - imgX * newScale;
        var targetPanY = focalY - imgY * newScale;
        var nw = naturalW();
        var nh = naturalH();

        panX = clampPanX(targetPanX, nw * newScale);
        panY = clampPanY(targetPanY, nh * newScale);

        interactionOccurred();
    }

    function panBy(dx, dy) {
        userInteracted = true;
        var nw = naturalW();
        var nh = naturalH();
        panX = clampPanX(panX + dx, nw * zoomFactor);
        panY = clampPanY(panY + dy, nh * zoomFactor);
        interactionOccurred();
    }

    Rectangle {
        anchors.fill: parent
        color: theme.darkerBackground
    }

    // Image viewport container
    Item {
        id: transformContainer
        x: root.panX
        y: root.panY
        width: Math.max(1, root.naturalW() * root.zoomFactor)
        height: Math.max(1, root.naturalH() * root.zoomFactor)

        Image {
            id: mainImage
            anchors.fill: parent
            visible: !root.isAnimated
            source: (!root.isAnimated && root.source) ? (root.source.indexOf("file://") === 0 ? root.source : ("file://" + root.source)) : ""
            fillMode: Image.PreserveAspectFit
            asynchronous: true
            cache: false
            autoTransform: true
            smooth: root.zoomFactor <= 1.5
            mipmap: root.zoomFactor <= 1.5

            onStatusChanged: {
                if (status === Image.Ready) {
                    if (sourceSize.width > 0) {
                        root.imgNaturalWidth = sourceSize.width;
                        root.imgNaturalHeight = sourceSize.height;
                        if (!root.isEditing && !root.userInteracted) {
                            root.resetView();
                        }
                    }
                }
            }
        }

        AnimatedImage {
            id: animImage
            anchors.fill: parent
            visible: root.isAnimated
            source: (root.isAnimated && root.source) ? (root.source.indexOf("file://") === 0 ? root.source : ("file://" + root.source)) : ""
            fillMode: Image.PreserveAspectFit
            asynchronous: true
            cache: false
            autoTransform: true
            smooth: root.zoomFactor <= 1.5
            mipmap: root.zoomFactor <= 1.5
            playing: true

            onStatusChanged: {
                if (status === AnimatedImage.Ready) {
                    if (sourceSize.width > 0) {
                        root.imgNaturalWidth = sourceSize.width;
                        root.imgNaturalHeight = sourceSize.height;
                        if (!root.isEditing && !root.userInteracted) {
                            root.resetView();
                        }
                    }
                }
            }
        }
    }

    // Mouse interactive area for Pan and Zoom
    MouseArea {
        id: mouseArea
        anchors.fill: parent
        hoverEnabled: true
        acceptedButtons: Qt.LeftButton | Qt.MiddleButton

        property real startMouseX: 0
        property real startMouseY: 0
        property real startPanX: 0
        property real startPanY: 0
        property bool isDragging: false

        onPressed: mouse => {
            root.interactionOccurred();
            if (mouse.button === Qt.LeftButton || mouse.button === Qt.MiddleButton) {
                startMouseX = mouse.x;
                startMouseY = mouse.y;
                startPanX = root.panX;
                startPanY = root.panY;
                isDragging = true;
            }
        }

        onPositionChanged: mouse => {
            root.interactionOccurred();
            if (isDragging) {
                var nw = root.naturalW();
                var nh = root.naturalH();
                root.panX = root.clampPanX(startPanX + (mouse.x - startMouseX), nw * root.zoomFactor);
                root.panY = root.clampPanY(startPanY + (mouse.y - startMouseY), nh * root.zoomFactor);
            }
        }

        onReleased: mouse => {
            isDragging = false;
        }

        onWheel: wheel => {
            root.interactionOccurred();
            var dy = wheel.angleDelta.y;
            if (dy === 0) dy = wheel.pixelDelta.y * 8;
            if (dy === 0) return;
            // Proportional zoom factor: standard mouse wheel click (120 units) gives ~1.12x
            // Fine-grained touchpad events (5-20 units) zoom smoothly by ~1.005x - 1.02x
            var factor = Math.exp(dy / 1000.0);
            root.zoomTo(root.zoomFactor * factor, wheel.x, wheel.y);
            wheel.accepted = true;
        }

        onDoubleClicked: mouse => {
            root.interactionOccurred();
            if (Math.abs(root.zoomFactor - root.fitScale()) < 0.01) {
                root.setZoom100();
            } else {
                root.resetView();
            }
            root.doubleClicked();
        }
    }
}
