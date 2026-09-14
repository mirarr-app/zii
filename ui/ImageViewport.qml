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

    clip: true

    onSourceChanged: {
        resetView();
    }

    onWidthChanged: {
        if (zoomFactor === fitScale()) {
            resetView();
        }
    }

    onHeightChanged: {
        if (zoomFactor === fitScale()) {
            resetView();
        }
    }

    function fitScale() {
        if (imgNaturalWidth <= 0 || imgNaturalHeight <= 0 || width <= 0 || height <= 0) {
            return 1.0;
        }
        var scaleX = width / imgNaturalWidth;
        var scaleY = height / imgNaturalHeight;
        return Math.min(scaleX, scaleY, 1.0);
    }

    function resetView() {
        var fit = fitScale();
        zoomFactor = fit;
        panX = (width - imgNaturalWidth * fit) / 2;
        panY = (height - imgNaturalHeight * fit) / 2;
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

        zoomFactor = newScale;
        panX = focalX - imgX * newScale;
        panY = focalY - imgY * newScale;

        interactionOccurred();
    }

    function panBy(dx, dy) {
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
        width: root.imgNaturalWidth * root.zoomFactor
        height: root.imgNaturalHeight * root.zoomFactor

        Image {
            id: mainImage
            anchors.fill: parent
            source: root.source ? ("file://" + root.source) : ""
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
