// Written by `cargo run --example grammars`. For each style, the colour of
// ordinary text and of the background, then each kind's colour and the
// font style its light theme gives it. Each colour is the light theme's,
// then the dark's.

use super::Style;

#[rustfmt::skip]
pub(super) const STYLES: &[Style] = &[
    Style {
        name: "ocean",
        text: ["#4f5b66", "#c0c5ce"],
        background: ["#eff1f5", "#2b303b"],
        kinds: [
            (["#a7adba", "#65737e"], ""), // comment
            (["#b48ead", "#b48ead"], ""), // keyword
            (["#a3be8c", "#a3be8c"], ""), // string
            (["#d08770", "#d08770"], ""), // number
            (["#d08770", "#d08770"], ""), // constant
            (["#8fa1b3", "#8fa1b3"], ""), // function
            (["#d08770", "#ebcb8b"], ""), // type
            (["#bf616a", "#bf616a"], ""), // variable
            (["#bf616a", "#bf616a"], ""), // property
            (["#bf616a", "#bf616a"], ""), // tag
            (["#d08770", "#d08770"], ""), // attribute
            (["#4f5b66", "#c0c5ce"], ""), // operator
            (["#4f5b66", "#c0c5ce"], ""), // punctuation
            (["#8fa1b3", "#8fa1b3"], ""), // heading
            (["#b48ead", "#b48ead"], "italic"), // emphasis
            (["#d08770", "#ebcb8b"], "bold"), // strong
            (["#d08770", "#d08770"], ""), // link
            (["#a3be8c", "#a3be8c"], ""), // inserted
            (["#bf616a", "#bf616a"], ""), // deleted
        ],
    },
    Style {
        name: "gruvbox",
        text: ["#3c3836", "#ebdbb2"],
        background: ["#fbf1c7", "#282828"],
        kinds: [
            (["#928374", "#928374"], "italic"), // comment
            (["#9d0006", "#fb4934"], ""), // keyword
            (["#79740e", "#b8bb26"], ""), // string
            (["#8f3f71", "#d3869b"], ""), // number
            (["#8f3f71", "#d3869b"], ""), // constant
            (["#b57614", "#fabd2f"], ""), // function
            (["#b57614", "#fabd2f"], ""), // type
            (["#076678", "#83a598"], ""), // variable
            (["#076678", "#83a598"], ""), // property
            (["#427b58", "#8ec07c"], ""), // tag
            (["#b57614", "#fabd2f"], ""), // attribute
            (["#427b58", "#8ec07c"], ""), // operator
            (["#7c6f64", "#a89984"], ""), // punctuation
            (["#9d0006", "#fb4934"], "bold"), // heading
            (["#3c3836", "#ebdbb2"], "italic"), // emphasis
            (["#af3a03", "#fe8019"], "bold"), // strong
            (["#8f3f71", "#d3869b"], "underline"), // link
            (["#79740e", "#b8bb26"], ""), // inserted
            (["#d65d0e", "#d65d0e"], ""), // deleted
        ],
    },
    Style {
        name: "solarized",
        text: ["#657b83", "#839496"],
        background: ["#fdf6e3", "#002b36"],
        kinds: [
            (["#93a1a1", "#586e75"], "italic"), // comment
            (["#859900", "#859900"], ""), // keyword
            (["#2aa198", "#2aa198"], ""), // string
            (["#d33682", "#d33682"], ""), // number
            (["#b58900", "#b58900"], ""), // constant
            (["#268bd2", "#268bd2"], ""), // function
            (["#cb4b16", "#cb4b16"], ""), // type
            (["#268bd2", "#268bd2"], ""), // variable
            (["#268bd2", "#268bd2"], ""), // property
            (["#268bd2", "#268bd2"], ""), // tag
            (["#93a1a1", "#93a1a1"], ""), // attribute
            (["#859900", "#859900"], ""), // operator
            (["#657b83", "#839496"], ""), // punctuation
            (["#268bd2", "#268bd2"], "bold"), // heading
            (["#d33682", "#d33682"], "italic"), // emphasis
            (["#d33682", "#d33682"], "bold"), // strong
            (["#657b83", "#839496"], ""), // link
            (["#859900", "#859900"], ""), // inserted
            (["#dc322f", "#dc322f"], ""), // deleted
        ],
    },
    Style {
        name: "catppuccin",
        text: ["#4c4f69", "#cdd6f4"],
        background: ["#eff1f5", "#1e1e2e"],
        kinds: [
            (["#7c7f93", "#9399b2"], "italic"), // comment
            (["#8839ef", "#cba6f7"], ""), // keyword
            (["#40a02b", "#a6e3a1"], ""), // string
            (["#fe640b", "#fab387"], ""), // number
            (["#d20f39", "#f38ba8"], ""), // constant
            (["#1e66f5", "#89b4fa"], "italic"), // function
            (["#df8e1d", "#f9e2af"], "italic"), // type
            (["#4c4f69", "#cdd6f4"], ""), // variable
            (["#4c4f69", "#cdd6f4"], ""), // property
            (["#1e66f5", "#89b4fa"], ""), // tag
            (["#df8e1d", "#f9e2af"], ""), // attribute
            (["#179299", "#94e2d5"], ""), // operator
            (["#7c7f93", "#9399b2"], ""), // punctuation
            (["#d20f39", "#f38ba8"], ""), // heading
            (["#d20f39", "#f38ba8"], "italic"), // emphasis
            (["#d20f39", "#f38ba8"], "bold"), // strong
            (["#1e66f5", "#89b4fa"], ""), // link
            (["#40a02b", "#a6e3a1"], ""), // inserted
            (["#d20f39", "#f38ba8"], ""), // deleted
        ],
    },
    Style {
        name: "one-half",
        text: ["#383a42", "#dcdfe4"],
        background: ["#fafafa", "#282c34"],
        kinds: [
            (["#a0a1a7", "#5c6370"], ""), // comment
            (["#a626a4", "#c678dd"], ""), // keyword
            (["#50a14f", "#98c379"], ""), // string
            (["#c18401", "#e5c07b"], ""), // number
            (["#c18401", "#e5c07b"], ""), // constant
            (["#0184bc", "#61afef"], ""), // function
            (["#c18401", "#e5c07b"], ""), // type
            (["#e45649", "#e06c75"], ""), // variable
            (["#e45649", "#e06c75"], ""), // property
            (["#e45649", "#e06c75"], ""), // tag
            (["#c18401", "#e5c07b"], ""), // attribute
            (["#a626a4", "#c678dd"], ""), // operator
            (["#383a42", "#dcdfe4"], ""), // punctuation
            (["#0184bc", "#61afef"], ""), // heading
            (["#a626a4", "#c678dd"], ""), // emphasis
            (["#a626a4", "#c678dd"], ""), // strong
            (["#50a14f", "#98c379"], ""), // link
            (["#98c379", "#98c379"], ""), // inserted
            (["#e06c75", "#e06c75"], ""), // deleted
        ],
    },
];
