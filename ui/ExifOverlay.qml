import QtQuick

Rectangle {
    id: root

    required property var theme
    property bool active: false
    property var exifData: null

    signal closed()

    visible: active
    opacity: active ? 1.0 : 0.0
    scale: active ? 1.0 : 0.96
    Behavior on opacity { NumberAnimation { duration: 180; easing.type: Easing.OutCubic } }
    Behavior on scale { NumberAnimation { duration: 180; easing.type: Easing.OutBack } }

    anchors.fill: parent
    color: Qt.rgba(0, 0, 0, 0.65)

    MouseArea {
        anchors.fill: parent
        onClicked: root.closed()
    }

    Rectangle {
        id: modalCard
        anchors.centerIn: parent
        width: Math.min(800, parent.width - 48)
        height: Math.min(600, parent.height - 48)
        radius: 16
        color: Qt.rgba(theme.darkerBackground.r, theme.darkerBackground.g, theme.darkerBackground.b, 0.96)
        border.color: theme.accent
        border.width: 1.5

        MouseArea {
            anchors.fill: parent // Prevent clicks inside modal from dismissing
        }

        Column {
            anchors.fill: parent
            anchors.margins: 26
            spacing: 18

            // Header Bar
            Item {
                width: parent.width
                height: 28

                Row {
                    anchors.left: parent.left
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 14

                    Rectangle {
                        width: tagLabel.implicitWidth + 16
                        height: 24
                        radius: 12
                        color: theme.accent
                        anchors.verticalCenter: parent.verticalCenter

                        Text {
                            id: tagLabel
                            anchors.centerIn: parent
                            text: "EXIF"
                            color: theme.darkerBackground
                            font.family: theme.fontFamily
                            font.pixelSize: 11
                            font.bold: true
                        }
                    }

                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        text: "Metadata & Image Inspector"
                        color: theme.brightForeground
                        font.family: theme.fontFamily
                        font.pixelSize: 17
                        font.bold: true
                    }
                }

                Text {
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    text: "Press e, x, or Esc to close"
                    color: theme.darkForeground
                    font.family: theme.fontFamily
                    font.pixelSize: 11
                }
            }

            // Separator line
            Rectangle {
                width: parent.width
                height: 1
                color: Qt.rgba(theme.muted.r, theme.muted.g, theme.muted.b, 0.25)
            }

            // Scrollable Content
            Flickable {
                width: parent.width
                height: parent.height - 76
                contentWidth: width
                contentHeight: contentColumn.implicitHeight
                clip: true

                Column {
                    id: contentColumn
                    width: parent.width
                    spacing: 22

                    // Row of Columns: Left (Camera & Exposure) and Right (File & Optics)
                    Row {
                        width: parent.width
                        spacing: 24

                        // Column 1: Camera & Optics + Exposure Settings
                        Column {
                            width: (parent.width - 24) / 2
                            spacing: 18

                            // Section: Camera & Optics
                            Text {
                                text: "CAMERA & OPTICS"
                                color: theme.accent
                                font.family: theme.fontFamily
                                font.pixelSize: 12
                                font.bold: true
                                font.letterSpacing: 1.0
                            }

                            Column {
                                width: parent.width
                                spacing: 8

                                ExifField {
                                    label: "Make"
                                    value: root.exifData ? (root.exifData.make || "—") : "—"
                                }
                                ExifField {
                                    label: "Model"
                                    value: root.exifData ? (root.exifData.model || "—") : "—"
                                }
                                ExifField {
                                    label: "Lens"
                                    value: root.exifData ? (root.exifData.lens_model || "—") : "—"
                                }
                                ExifField {
                                    label: "Focal Length"
                                    value: root.exifData ? (root.exifData.focal_length || "—") : "—"
                                }
                            }

                            // Section: Exposure Settings
                            Text {
                                text: "EXPOSURE SETTINGS"
                                color: theme.accent
                                font.family: theme.fontFamily
                                font.pixelSize: 12
                                font.bold: true
                                font.letterSpacing: 1.0
                            }

                            Column {
                                width: parent.width
                                spacing: 8

                                ExifField {
                                    label: "Shutter Speed"
                                    value: root.exifData ? (root.exifData.shutter_speed || "—") : "—"
                                }
                                ExifField {
                                    label: "Aperture"
                                    value: root.exifData ? (root.exifData.f_number || "—") : "—"
                                }
                                ExifField {
                                    label: "ISO"
                                    value: root.exifData ? (root.exifData.iso || "—") : "—"
                                }
                                ExifField {
                                    label: "Exposure Bias"
                                    value: root.exifData ? (root.exifData.exposure_bias || "—") : "—"
                                }
                                ExifField {
                                    label: "Flash"
                                    value: root.exifData ? (root.exifData.flash || "—") : "—"
                                }
                            }
                        }

                        // Column 2: File & Dimensions
                        Column {
                            width: (parent.width - 24) / 2
                            spacing: 18

                            // Section: File & Dimensions
                            Text {
                                text: "FILE & DIMENSIONS"
                                color: theme.accent
                                font.family: theme.fontFamily
                                font.pixelSize: 12
                                font.bold: true
                                font.letterSpacing: 1.0
                            }

                            Column {
                                width: parent.width
                                spacing: 8

                                ExifField {
                                    label: "Dimensions"
                                    value: root.exifData ? (root.exifData.dimensions || "—") : "—"
                                }
                                ExifField {
                                    label: "Format"
                                    value: root.exifData ? (root.exifData.format || "—") : "—"
                                }
                                ExifField {
                                    label: "File Size"
                                    value: root.exifData ? (root.exifData.file_size || "—") : "—"
                                }
                                ExifField {
                                    label: "Date Taken"
                                    value: root.exifData ? (root.exifData.date_time || "—") : "—"
                                }
                                ExifField {
                                    label: "Filename"
                                    value: root.exifData ? (root.exifData.filename || "—") : "—"
                                }
                            }
                        }
                    }

                    // Full Path Card at bottom
                    Rectangle {
                        width: parent.width
                        height: pathRow.implicitHeight + 16
                        radius: 8
                        color: theme.surface
                        border.color: theme.muted
                        border.width: 1

                        Row {
                            id: pathRow
                            anchors.left: parent.left
                            anchors.right: parent.right
                            anchors.leftMargin: 12
                            anchors.rightMargin: 12
                            anchors.verticalCenter: parent.verticalCenter
                            spacing: 10

                            Text {
                                text: "Full Path"
                                color: theme.accent
                                font.family: theme.fontFamily
                                font.pixelSize: 11
                                font.bold: true
                                anchors.verticalCenter: parent.verticalCenter
                            }

                            Text {
                                width: parent.width - 80
                                text: root.exifData ? (root.exifData.path || "—") : "—"
                                color: theme.lightForeground
                                font.family: theme.fontFamily
                                font.pixelSize: 11
                                elide: Text.ElideMiddle
                                anchors.verticalCenter: parent.verticalCenter
                            }
                        }
                    }
                }
            }
        }
    }

    // Helper row component for metadata fields
    component ExifField: Row {
        id: fieldRow
        property string label: ""
        property string value: "—"
        property real labelWidth: 100
        spacing: 10
        width: parent.width

        Text {
            width: fieldRow.labelWidth
            text: fieldRow.label
            color: theme.darkForeground
            font.family: theme.fontFamily
            font.pixelSize: 12
            anchors.verticalCenter: parent.verticalCenter
        }

        Text {
            width: parent.width - fieldRow.labelWidth - fieldRow.spacing
            text: fieldRow.value || "—"
            color: (fieldRow.value && fieldRow.value !== "—") ? theme.brightForeground : theme.muted
            font.family: theme.fontFamily
            font.pixelSize: 12
            font.bold: (fieldRow.value && fieldRow.value !== "—")
            elide: Text.ElideRight
            anchors.verticalCenter: parent.verticalCenter
        }
    }
}
