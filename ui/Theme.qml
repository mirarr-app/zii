import QtQuick

QtObject {
    id: root

    property string name: "Omarchy Dark"
    property color background: "#1e1e2e"
    property color darkBackground: "#181825"
    property color darkerBackground: "#11111b"
    property color lighterBackground: "#313244"
    property color surface: "#252739"
    property color surfaceElevated: "#2d3043"

    property color foreground: "#cdd6f4"
    property color darkForeground: "#6c7086"
    property color lightForeground: "#bac2de"
    property color brightForeground: "#ffffff"

    property color accent: "#89b4fa"
    property color selection: "#45475a"
    property color muted: "#585b70"

    property color red: "#f38ba8"
    property color yellow: "#f9e2af"
    property color green: "#a6e3a1"
    property color cyan: "#94e2d5"
    property color blue: "#89b4fa"
    property color magenta: "#f5c2e7"

    property string fontFamily: "JetBrains Mono, Inter, Cantarell, Sans Serif"
    property int baseFontSize: 13

    function apply(themeObj) {
        if (!themeObj) return;
        if (themeObj.name) root.name = themeObj.name;
        if (themeObj.background) root.background = themeObj.background;
        if (themeObj.dark_background) root.darkBackground = themeObj.dark_background;
        if (themeObj.darker_background) root.darkerBackground = themeObj.darker_background;
        if (themeObj.lighter_background) {
            root.lighterBackground = themeObj.lighter_background;
            root.surface = themeObj.lighter_background;
        }
        if (themeObj.foreground) root.foreground = themeObj.foreground;
        if (themeObj.dark_foreground) root.darkForeground = themeObj.dark_foreground;
        if (themeObj.light_foreground) root.lightForeground = themeObj.light_foreground;
        if (themeObj.bright_foreground) root.brightForeground = themeObj.bright_foreground;
        if (themeObj.accent) root.accent = themeObj.accent;
        if (themeObj.selection) root.selection = themeObj.selection;
        if (themeObj.muted) root.muted = themeObj.muted;
        if (themeObj.red) root.red = themeObj.red;
        if (themeObj.yellow) root.yellow = themeObj.yellow;
        if (themeObj.green) root.green = themeObj.green;
        if (themeObj.cyan) root.cyan = themeObj.cyan;
        if (themeObj.blue) root.blue = themeObj.blue;
        if (themeObj.magenta) root.magenta = themeObj.magenta;
    }
}
