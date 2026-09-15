import QtQuick

Rectangle {
    id: root

    required property var theme
    property bool active: false

    signal saveOverwrite()
    signal saveCopy()
    signal cancel()

    visible: active
    anchors.fill: parent
    color: Qt.rgba(0, 0, 0, 0.55)

    MouseArea {
        anchors.fill: parent
        onClicked: root.cancel()
    }

    Rectangle {
        id: card
        anchors.centerIn: parent
        width: 380
        height: 220
        radius: 14
        color: theme.darkerBackground
        border.color: theme.accent
        border.width: 1

        MouseArea {
            anchors.fill: parent // Prevent clicks from passing through
        }

        Column {
            anchors.fill: parent
            anchors.margins: 24
            spacing: 16

            Text {
                text: "Save Image Edits"
                color: theme.brightForeground
                font.family: theme.fontFamily
                font.pixelSize: 16
                font.bold: true
            }

            Text {
                text: "How would you like to save your edits?"
                color: theme.lightForeground
                font.family: theme.fontFamily
                font.pixelSize: 12
            }

            Row {
                spacing: 12
                anchors.horizontalCenter: parent.horizontalCenter

                // Overwrite button
                Rectangle {
                    width: 155
                    height: 40
                    radius: 8
                    color: theme.surface
                    border.color: theme.muted
                    border.width: 1

                    Row {
                        anchors.centerIn: parent
                        spacing: 8
                        Text {
                            text: "Overwrite"
                            color: theme.foreground
                            font.family: theme.fontFamily
                            font.bold: true
                            font.pixelSize: 12
                        }
                        Rectangle {
                            height: 18
                            width: 18
                            radius: 4
                            color: Qt.rgba(1, 1, 1, 0.1)
                            Text {
                                anchors.centerIn: parent
                                text: "w"
                                color: theme.accent
                                font.bold: true
                                font.pixelSize: 10
                            }
                        }
                    }

                    MouseArea {
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: root.saveOverwrite()
                    }
                }

                // Save copy button
                Rectangle {
                    width: 155
                    height: 40
                    radius: 8
                    color: theme.accent

                    Row {
                        anchors.centerIn: parent
                        spacing: 8
                        Text {
                            text: "Save Copy"
                            color: theme.darkerBackground
                            font.family: theme.fontFamily
                            font.bold: true
                            font.pixelSize: 12
                        }
                        Rectangle {
                            height: 18
                            width: 18
                            radius: 4
                            color: Qt.rgba(0, 0, 0, 0.2)
                            Text {
                                anchors.centerIn: parent
                                text: "s"
                                color: theme.darkerBackground
                                font.bold: true
                                font.pixelSize: 10
                            }
                        }
                    }

                    MouseArea {
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: root.saveCopy()
                    }
                }
            }

            Text {
                anchors.horizontalCenter: parent.horizontalCenter
                text: "Press Esc to cancel"
                color: theme.darkForeground
                font.family: theme.fontFamily
                font.pixelSize: 11
            }
        }
    }
}
