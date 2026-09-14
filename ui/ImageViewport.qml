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

    signal interactionOccurred()
    signal doubleClicked()

    property bool userInteracted: false

    clip: true

    onSourceChanged: {
        userInteracted = false;
        resetView();
    }

    onWidthChanged: {
        if (!userInteracted && width > 0 && height > 0) {
            resetView();
        }
    }

    onHeightChanged: {
        if (!userInteracted && width > 0 && height > 0) {
            resetView();
        }
    }

    function fitScale() {
        var nw = imgNaturalWidth > 0 ? imgNaturalWidth : (mainImage.sourceSize.width > 0 ? mainImage.sourceSize.width : 0);
        var nh = imgNaturalHeight > 0 ? imgNaturalHeight : (mainImage.sourceSize.height > 0 ? mainImage.sourceSize.height : 0);

        if (nw <= 0 || nh <= 0 || width <= 0 || height <= 0) {
            return 1.0;
        }
        var scaleX = width / nw;
        var scaleY = height / nh;
        return Math.min(scaleX, scaleY, 1.0);
    }

    function resetView() {
        var nw = imgNaturalWidth > 0 ? imgNaturalWidth : (mainImage.sourceSize.width > 0 ? mainImage.sourceSize.width : 0);
        var nh = imgNaturalHeight > 0 ? imgNaturalHeight : (mainImage.sourceSize.height > 0 ? mainImage.sourceSize.height : 0);

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
        panX = focalX - imgX * newScale;
        panY = focalY - imgY * newScale;

        interactionOccurred();
    }

    function panBy(dx, dy) {
        userInteracted = true;
        panX += dx;
        panY += dy;
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
        width: Math.max(1, (root.imgNaturalWidth > 0 ? root.imgNaturalWidth : mainImage.sourceSize.width) * root.zoomFactor)
        height: Math.max(1, (root.imgNaturalHeight > 0 ? root.imgNaturalHeight : mainImage.sourceSize.height) * root.zoomFactor)

        Image {
            id: mainImage
            anchors.fill: parent
            source: root.source ? (root.source.indexOf("file://") === 0 ? root.source : ("file://" + root.source)) : ""
            fillMode: Image.PreserveAspectFit
            asynchronous: true
            cache: false
            smooth: true
            mipmap: true

            onStatusChanged: {
                if (status === Image.Ready) {
                    if (sourceSize.width > 0) {
                        root.imgNaturalWidth = sourceSize.width;
                        root.imgNaturalHeight = sourceSize.height;
                        root.resetView();
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
                root.panX = startPanX + (mouse.x - startMouseX);
                root.panY = startPanY + (mouse.y - startMouseY);
            }
        }

        onReleased: mouse => {
            isDragging = false;
        }

        onWheel: wheel => {
            root.interactionOccurred();
            var factor = wheel.angleDelta.y > 0 ? 1.15 : 0.85;
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
