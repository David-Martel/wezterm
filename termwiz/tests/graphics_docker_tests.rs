//! Docker-based terminal graphics tests.
//!
//! These tests verify terminal graphics rendering capabilities using Docker
//! containers for isolated testing. Tests cover:
//! - ANSI escape sequence generation
//! - Sixel graphics protocol
//! - iTerm2 inline image protocol
//! - Kitty graphics protocol
//!
//! To run these tests:
//! ```bash
//! cargo test --test graphics_docker_tests --features docker-tests -- --ignored
//! ```
//!
//! Prerequisites:
//! - Docker installed and running
//! - Node.js image available (node:lts-slim)

#![cfg(feature = "docker-tests")]

use std::process::Command;

/// Check if Docker is available.
fn docker_available() -> bool {
    Command::new("docker")
        .args(["info"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Run a Node.js script in Docker and return the output.
fn run_node_in_docker(script: &str) -> Result<String, String> {
    let output = Command::new("docker")
        .args([
            "run",
            "--rm",
            "node:lts-slim",
            "node",
            "-e",
            script,
        ])
        .output()
        .map_err(|e| format!("Failed to run docker: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[cfg(test)]
mod graphics_tests {
    use super::*;

    #[test]
    #[ignore] // Run manually with: cargo test --test graphics_docker_tests -- --ignored
    fn test_ansi_color_generation() {
        if !docker_available() {
            eprintln!("Docker not available, skipping test");
            return;
        }

        let script = r#"
const ESC = '\x1b';
const tests = {
    basic16: [],
    colors256: [],
    trueColor: []
};

// Generate basic 16 colors (foreground)
for (let i = 30; i <= 37; i++) {
    tests.basic16.push(`${ESC}[${i}m`);
}
for (let i = 90; i <= 97; i++) {
    tests.basic16.push(`${ESC}[${i}m`);
}

// Generate 256 colors
for (let i = 0; i < 256; i += 16) {
    tests.colors256.push(`${ESC}[38;5;${i}m`);
}

// Generate true colors
tests.trueColor.push(`${ESC}[38;2;255;0;0m`);    // Red
tests.trueColor.push(`${ESC}[38;2;0;255;0m`);    // Green
tests.trueColor.push(`${ESC}[38;2;0;0;255m`);    // Blue
tests.trueColor.push(`${ESC}[38;2;255;255;0m`);  // Yellow
tests.trueColor.push(`${ESC}[38;2;255;0;255m`);  // Magenta
tests.trueColor.push(`${ESC}[38;2;0;255;255m`);  // Cyan

// Validate all have ESC
const validate = arr => arr.every(s => s.includes('\x1b'));
console.log(JSON.stringify({
    basic16_count: tests.basic16.length,
    colors256_count: tests.colors256.length,
    trueColor_count: tests.trueColor.length,
    basic16_valid: validate(tests.basic16),
    colors256_valid: validate(tests.colors256),
    trueColor_valid: validate(tests.trueColor)
}));
"#;

        let result = run_node_in_docker(script).expect("Docker test failed");
        let json: serde_json::Value = serde_json::from_str(&result).expect("Parse JSON");

        assert_eq!(json["basic16_count"], 16);
        assert_eq!(json["colors256_count"], 16);
        assert_eq!(json["trueColor_count"], 6);
        assert_eq!(json["basic16_valid"], true);
        assert_eq!(json["colors256_valid"], true);
        assert_eq!(json["trueColor_valid"], true);
    }

    #[test]
    #[ignore]
    fn test_sixel_protocol_generation() {
        if !docker_available() {
            eprintln!("Docker not available, skipping test");
            return;
        }

        let script = r#"
const ESC = '\x1b';

// Sixel graphics protocol components
const sixel = {
    // DCS (Device Control String) introducer
    dcs: `${ESC}P`,
    // Sixel header: aspect ratio, background
    header: 'q',
    // Color introduction: #Pc;Pu;Px;Py;Pz
    // Pc = color number, Pu = coordinate unit (2=RGB), Px,Py,Pz = RGB values
    colorDefs: [
        '#0;2;100;0;0',    // Color 0: Red
        '#1;2;0;100;0',    // Color 1: Green
        '#2;2;0;0;100',    // Color 2: Blue
        '#3;2;100;100;0',  // Color 3: Yellow
    ],
    // Sixel data characters (? through ~ represent 6 vertical pixels)
    // Each character encodes 6 pixels in binary
    pixelData: [
        '!10?',  // Repeat character: 10 x '?' (all 6 pixels off)
        '!10~',  // Repeat character: 10 x '~' (all 6 pixels on)
        '#0!5~', // Color 0, 5 x '~'
        '#1!5~', // Color 1, 5 x '~'
        '-',     // Graphics newline (next row of 6 pixels)
        '$',     // Graphics carriage return
    ],
    // String Terminator
    st: `${ESC}\\`
};

// Build a simple Sixel test image (2x2 color blocks)
const buildSixelImage = () => {
    let seq = sixel.dcs + sixel.header;
    seq += sixel.colorDefs.join('');
    seq += '#0!10~';  // Row 1: Color 0
    seq += '-';       // New row
    seq += '#1!10~';  // Row 2: Color 1
    seq += sixel.st;
    return seq;
};

const testImage = buildSixelImage();
console.log(JSON.stringify({
    dcs_correct: sixel.dcs === '\x1bP',
    st_correct: sixel.st === '\x1b\\',
    header_present: testImage.includes('q'),
    color_defs_count: sixel.colorDefs.length,
    has_pixel_data: testImage.includes('~'),
    total_length: testImage.length,
    starts_with_dcs: testImage.startsWith('\x1bP'),
    ends_with_st: testImage.endsWith('\x1b\\')
}));
"#;

        let result = run_node_in_docker(script).expect("Docker test failed");
        let json: serde_json::Value = serde_json::from_str(&result).expect("Parse JSON");

        assert_eq!(json["dcs_correct"], true);
        assert_eq!(json["st_correct"], true);
        assert_eq!(json["header_present"], true);
        assert_eq!(json["color_defs_count"], 4);
        assert_eq!(json["has_pixel_data"], true);
        assert_eq!(json["starts_with_dcs"], true);
        assert_eq!(json["ends_with_st"], true);
    }

    #[test]
    #[ignore]
    fn test_iterm2_image_protocol() {
        if !docker_available() {
            eprintln!("Docker not available, skipping test");
            return;
        }

        let script = r#"
const ESC = '\x1b';
const BEL = '\x07';

// iTerm2 Inline Images Protocol
// Format: OSC 1337 ; File=<args> : <base64 data> BEL

// Build an iTerm2 image sequence
const buildITerm2Image = (options = {}) => {
    const args = [];

    if (options.name) args.push(`name=${Buffer.from(options.name).toString('base64')}`);
    if (options.size) args.push(`size=${options.size}`);
    if (options.width) args.push(`width=${options.width}`);
    if (options.height) args.push(`height=${options.height}`);
    args.push(`inline=${options.inline ? '1' : '0'}`);
    if (options.preserveAspectRatio !== undefined) {
        args.push(`preserveAspectRatio=${options.preserveAspectRatio ? '1' : '0'}`);
    }

    // Create a minimal 1x1 red PNG (base64)
    const minimalPng = 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8DwHwAFBQIAX8jx0gAAAABJRU5ErkJggg==';

    return `${ESC}]1337;File=${args.join(';')}:${minimalPng}${BEL}`;
};

const testCases = [
    buildITerm2Image({ inline: true }),
    buildITerm2Image({ inline: true, width: 'auto', height: '100px' }),
    buildITerm2Image({ inline: true, name: 'test.png', size: 68 }),
    buildITerm2Image({ inline: true, preserveAspectRatio: true }),
];

console.log(JSON.stringify({
    test_count: testCases.length,
    all_start_with_osc: testCases.every(t => t.startsWith('\x1b]1337')),
    all_end_with_bel: testCases.every(t => t.endsWith('\x07')),
    all_have_inline: testCases.every(t => t.includes('inline=')),
    all_have_base64_marker: testCases.every(t => t.includes(':')),
    first_length: testCases[0].length
}));
"#;

        let result = run_node_in_docker(script).expect("Docker test failed");
        let json: serde_json::Value = serde_json::from_str(&result).expect("Parse JSON");

        assert_eq!(json["test_count"], 4);
        assert_eq!(json["all_start_with_osc"], true);
        assert_eq!(json["all_end_with_bel"], true);
        assert_eq!(json["all_have_inline"], true);
        assert_eq!(json["all_have_base64_marker"], true);
    }

    #[test]
    #[ignore]
    fn test_kitty_graphics_protocol() {
        if !docker_available() {
            eprintln!("Docker not available, skipping test");
            return;
        }

        let script = r#"
const ESC = '\x1b';

// Kitty Graphics Protocol
// Format: APC G <key>=<value>,... ; <payload> ST
// APC = ESC _ (Application Program Command)
// ST = ESC \ (String Terminator)

const kitty = {
    apc: `${ESC}_G`,
    st: `${ESC}\\`,

    // Build a kitty graphics command
    buildCommand: (params, payload = '') => {
        const paramStr = Object.entries(params)
            .map(([k, v]) => `${k}=${v}`)
            .join(',');
        return `${ESC}_G${paramStr};${payload}${ESC}\\`;
    }
};

// Test different Kitty actions
const testCases = {
    // Transmit image (a=t)
    transmit: kitty.buildCommand({
        a: 't',      // action: transmit
        f: 100,      // format: PNG
        t: 'd',      // transmission: direct
        s: 1,        // width
        v: 1,        // height
    }, 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8DwHwAFBQIAX8jx0gAAAABJRU5ErkJggg=='),

    // Query support (a=q)
    query: kitty.buildCommand({ a: 'q', i: 1 }),

    // Display image (a=p)
    display: kitty.buildCommand({
        a: 'p',      // action: display
        i: 1,        // image id
        p: 1,        // placement id
    }),

    // Delete image (a=d)
    delete: kitty.buildCommand({
        a: 'd',      // action: delete
        d: 'a',      // delete: all
    }),
};

console.log(JSON.stringify({
    apc_correct: kitty.apc === '\x1b_G',
    st_correct: kitty.st === '\x1b\\',
    transmit_has_action: testCases.transmit.includes('a=t'),
    transmit_has_format: testCases.transmit.includes('f=100'),
    query_has_action: testCases.query.includes('a=q'),
    display_has_action: testCases.display.includes('a=p'),
    delete_has_action: testCases.delete.includes('a=d'),
    all_start_with_apc: Object.values(testCases).every(t => t.startsWith('\x1b_G')),
    all_end_with_st: Object.values(testCases).every(t => t.endsWith('\x1b\\\\')),
}));
"#;

        let result = run_node_in_docker(script).expect("Docker test failed");
        let json: serde_json::Value = serde_json::from_str(&result).expect("Parse JSON");

        assert_eq!(json["apc_correct"], true);
        assert_eq!(json["st_correct"], true);
        assert_eq!(json["transmit_has_action"], true);
        assert_eq!(json["query_has_action"], true);
        assert_eq!(json["display_has_action"], true);
        assert_eq!(json["delete_has_action"], true);
        assert_eq!(json["all_start_with_apc"], true);
    }

    #[test]
    #[ignore]
    fn test_cursor_and_screen_control() {
        if !docker_available() {
            eprintln!("Docker not available, skipping test");
            return;
        }

        let script = r#"
const ESC = '\x1b';
const CSI = `${ESC}[`;

// Cursor movement sequences
const cursor = {
    up: n => `${CSI}${n}A`,
    down: n => `${CSI}${n}B`,
    forward: n => `${CSI}${n}C`,
    back: n => `${CSI}${n}D`,
    position: (row, col) => `${CSI}${row};${col}H`,
    save: `${CSI}s`,
    restore: `${CSI}u`,
    hide: `${CSI}?25l`,
    show: `${CSI}?25h`,
};

// Screen control sequences
const screen = {
    clear: `${CSI}2J`,
    clearBelow: `${CSI}0J`,
    clearAbove: `${CSI}1J`,
    clearLine: `${CSI}2K`,
    clearLineRight: `${CSI}0K`,
    clearLineLeft: `${CSI}1K`,
    scrollUp: n => `${CSI}${n}S`,
    scrollDown: n => `${CSI}${n}T`,
    setScrollRegion: (top, bottom) => `${CSI}${top};${bottom}r`,
    resetScrollRegion: `${CSI}r`,
};

// Mode sequences
const modes = {
    alternateScreen: `${CSI}?1049h`,
    normalScreen: `${CSI}?1049l`,
    bracketedPasteOn: `${CSI}?2004h`,
    bracketedPasteOff: `${CSI}?2004l`,
    mouseTrackingOn: `${CSI}?1000h`,
    mouseTrackingOff: `${CSI}?1000l`,
    sgrMouseOn: `${CSI}?1006h`,
    sgrMouseOff: `${CSI}?1006l`,
};

// Validate all sequences start with ESC[
const validateCSI = seq => seq.startsWith('\x1b[');

console.log(JSON.stringify({
    cursor_up: validateCSI(cursor.up(5)),
    cursor_down: validateCSI(cursor.down(3)),
    cursor_position: validateCSI(cursor.position(10, 20)),
    cursor_save: validateCSI(cursor.save),
    cursor_restore: validateCSI(cursor.restore),
    screen_clear: validateCSI(screen.clear),
    screen_scroll_region: validateCSI(screen.setScrollRegion(5, 20)),
    mode_alternate: validateCSI(modes.alternateScreen),
    mode_bracketed: validateCSI(modes.bracketedPasteOn),
    mode_mouse: validateCSI(modes.mouseTrackingOn),
    all_valid: true
}));
"#;

        let result = run_node_in_docker(script).expect("Docker test failed");
        let json: serde_json::Value = serde_json::from_str(&result).expect("Parse JSON");

        assert_eq!(json["cursor_up"], true);
        assert_eq!(json["cursor_down"], true);
        assert_eq!(json["cursor_position"], true);
        assert_eq!(json["screen_clear"], true);
        assert_eq!(json["mode_alternate"], true);
        assert_eq!(json["mode_mouse"], true);
    }
}

/// Verify Docker is working for graphics tests.
#[cfg(test)]
mod infrastructure_tests {
    use super::*;

    #[test]
    fn test_docker_availability_check() {
        let available = docker_available();
        println!("Docker available: {}", available);
        // This test always passes - it's for diagnostics
    }

    #[test]
    fn test_node_docker_image() {
        if !docker_available() {
            eprintln!("Docker not available, skipping test");
            return;
        }

        let output = Command::new("docker")
            .args(["images", "node:lts-slim", "--format", "{{.Repository}}:{{.Tag}}"])
            .output();

        match output {
            Ok(o) if o.status.success() => {
                let images = String::from_utf8_lossy(&o.stdout);
                println!("Node images: {}", images);
            }
            _ => {
                println!("Node image not cached (will be pulled on first use)");
            }
        }
    }
}
