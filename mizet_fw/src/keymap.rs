use usbd_hid::descriptor::KeyboardUsage;

pub struct KeyInfo {
    pub keycode: KeyboardUsage,
    pub key: &'static str,
    pub shifted_key: &'static str,
    pub middle_key: Option<&'static str>,
}

pub static KEY_LENGTH: usize = 62;

pub fn get_next_idx(current_index: usize) -> usize {
    if current_index == KEY_LENGTH - 1 {
        0
    } else {
        current_index + 1
    }
}
pub fn get_prev_idx(current_index: usize) -> usize {
    if current_index == 0 {
        KEY_LENGTH - 1
    } else {
        current_index - 1
    }
}

pub static KEYMAP: [KeyInfo; KEY_LENGTH] = [
    // Alphabet keys
    KeyInfo {
        keycode: KeyboardUsage::KeyboardAa,
        shifted_key: "A",
        key: "a",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardBb,
        shifted_key: "B",
        key: "b",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardCc,
        shifted_key: "C",
        key: "c",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardDd,
        shifted_key: "D",
        key: "d",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardEe,
        shifted_key: "E",
        key: "e",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardFf,
        shifted_key: "F",
        key: "f",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardGg,
        shifted_key: "G",
        key: "g",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardHh,
        shifted_key: "H",
        key: "h",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardIi,
        shifted_key: "I",
        key: "i",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardJj,
        shifted_key: "J",
        key: "j",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardKk,
        shifted_key: "K",
        key: "k",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardLl,
        shifted_key: "L",
        key: "l",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardMm,
        shifted_key: "M",
        key: "m",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardNn,
        shifted_key: "N",
        key: "n",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardOo,
        shifted_key: "O",
        key: "o",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardPp,
        shifted_key: "P",
        key: "p",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardQq,
        shifted_key: "Q",
        key: "q",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardRr,
        shifted_key: "R",
        key: "r",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardSs,
        shifted_key: "S",
        key: "s",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardTt,
        shifted_key: "T",
        key: "t",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardUu,
        shifted_key: "U",
        key: "u",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardVv,
        shifted_key: "V",
        key: "v",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardWw,
        shifted_key: "W",
        key: "w",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardXx,
        shifted_key: "X",
        key: "x",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardYy,
        shifted_key: "Y",
        key: "y",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardZz,
        shifted_key: "Z",
        key: "z",
        middle_key: None,
    },
    // Number keys
    KeyInfo {
        keycode: KeyboardUsage::Keyboard1Exclamation,
        shifted_key: "!",
        key: "1",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::Keyboard2At,
        shifted_key: "@",
        key: "2",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::Keyboard3Hash,
        shifted_key: "#",
        key: "3",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::Keyboard4Dollar,
        shifted_key: "$",
        key: "4",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::Keyboard5Percent,
        shifted_key: "%",
        key: "5",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::Keyboard6Caret,
        shifted_key: "^",
        key: "6",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::Keyboard7Ampersand,
        shifted_key: "&",
        key: "7",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::Keyboard8Asterisk,
        shifted_key: "*",
        key: "8",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::Keyboard9OpenParens,
        shifted_key: "(",
        key: "9",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::Keyboard0CloseParens,
        shifted_key: ")",
        key: "0",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardEnter,
        shifted_key: "R",
        middle_key: Some("E"),
        key: "T",
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardEscape,
        shifted_key: "E",
        middle_key: Some("S"),
        key: "C",
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardBackspace,
        shifted_key: "B",
        middle_key: Some("S"),
        key: "P",
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardTab,
        shifted_key: "T",
        middle_key: Some("A"),
        key: "B",
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardSpacebar,
        shifted_key: "S",
        middle_key: Some("P"),
        key: "C",
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardDashUnderscore,
        shifted_key: "_",
        key: "-",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardEqualPlus,
        shifted_key: "+",
        key: "=",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardOpenBracketBrace,
        shifted_key: "{",
        key: "[",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardCloseBracketBrace,
        shifted_key: "}",
        key: "]",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardBackslashBar,
        shifted_key: "|",
        key: "\\",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardSemiColon,
        shifted_key: ":",
        key: ";",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardSingleDoubleQuote,
        shifted_key: "\"",
        key: "\'",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardBacktickTilde,
        shifted_key: "~",
        key: "`",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardCommaLess,
        shifted_key: "<",
        key: ",",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardPeriodGreater,
        shifted_key: ">",
        key: ".",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardSlashQuestion,
        shifted_key: "?",
        key: "/",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardF1,
        shifted_key: "F",
        key: "1",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardF2,
        shifted_key: "F",
        key: "2",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardF3,
        shifted_key: "F",
        key: "3",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardF4,
        shifted_key: "F",
        key: "4",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardF5,
        shifted_key: "F",
        key: "5",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardF6,
        shifted_key: "F",
        key: "6",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardF7,
        shifted_key: "F",
        key: "7",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardF8,
        shifted_key: "F",
        key: "8",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardF9,
        shifted_key: "F",
        key: "9",
        middle_key: None,
    },
    KeyInfo {
        keycode: KeyboardUsage::KeyboardF10,
        shifted_key: "F",
        key: "0",
        middle_key: None,
    },
];
