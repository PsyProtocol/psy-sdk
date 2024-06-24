
export const enum MonacoKeyMod {
	CtrlCmd = (1 << 11) >>> 0,
	Shift = (1 << 10) >>> 0,
	Alt = (1 << 9) >>> 0,
	WinCtrl = (1 << 8) >>> 0,
}

export const enum MonacoKeyCode {
  DependsOnKbLayout = -1,
  /**
   * Placed first to cover the 0 value of the enum.
   */
  Unknown = 0,
  Backspace = 1,
  Tab = 2,
  Enter = 3,
  Shift = 4,
  Ctrl = 5,
  Alt = 6,
  PauseBreak = 7,
  CapsLock = 8,
  Escape = 9,
  Space = 10,
  PageUp = 11,
  PageDown = 12,
  End = 13,
  Home = 14,
  LeftArrow = 15,
  UpArrow = 16,
  RightArrow = 17,
  DownArrow = 18,
  Insert = 19,
  Delete = 20,
  Digit0 = 21,
  Digit1 = 22,
  Digit2 = 23,
  Digit3 = 24,
  Digit4 = 25,
  Digit5 = 26,
  Digit6 = 27,
  Digit7 = 28,
  Digit8 = 29,
  Digit9 = 30,
  KeyA = 31,
  KeyB = 32,
  KeyC = 33,
  KeyD = 34,
  KeyE = 35,
  KeyF = 36,
  KeyG = 37,
  KeyH = 38,
  KeyI = 39,
  KeyJ = 40,
  KeyK = 41,
  KeyL = 42,
  KeyM = 43,
  KeyN = 44,
  KeyO = 45,
  KeyP = 46,
  KeyQ = 47,
  KeyR = 48,
  KeyS = 49,
  KeyT = 50,
  KeyU = 51,
  KeyV = 52,
  KeyW = 53,
  KeyX = 54,
  KeyY = 55,
  KeyZ = 56,
  Meta = 57,
  ContextMenu = 58,
  F1 = 59,
  F2 = 60,
  F3 = 61,
  F4 = 62,
  F5 = 63,
  F6 = 64,
  F7 = 65,
  F8 = 66,
  F9 = 67,
  F10 = 68,
  F11 = 69,
  F12 = 70,
  F13 = 71,
  F14 = 72,
  F15 = 73,
  F16 = 74,
  F17 = 75,
  F18 = 76,
  F19 = 77,
  F20 = 78,
  F21 = 79,
  F22 = 80,
  F23 = 81,
  F24 = 82,
  NumLock = 83,
  ScrollLock = 84,
  /**
   * Used for miscellaneous characters; it can vary by keyboard.
   * For the US standard keyboard, the ';:' key
   */
  Semicolon = 85,
  /**
   * For any country/region, the '+' key
   * For the US standard keyboard, the '=+' key
   */
  Equal = 86,
  /**
   * For any country/region, the ',' key
   * For the US standard keyboard, the ',<' key
   */
  Comma = 87,
  /**
   * For any country/region, the '-' key
   * For the US standard keyboard, the '-_' key
   */
  Minus = 88,
  /**
   * For any country/region, the '.' key
   * For the US standard keyboard, the '.>' key
   */
  Period = 89,
  /**
   * Used for miscellaneous characters; it can vary by keyboard.
   * For the US standard keyboard, the '/?' key
   */
  Slash = 90,
  /**
   * Used for miscellaneous characters; it can vary by keyboard.
   * For the US standard keyboard, the '`~' key
   */
  Backquote = 91,
  /**
   * Used for miscellaneous characters; it can vary by keyboard.
   * For the US standard keyboard, the '[{' key
   */
  BracketLeft = 92,
  /**
   * Used for miscellaneous characters; it can vary by keyboard.
   * For the US standard keyboard, the '\|' key
   */
  Backslash = 93,
  /**
   * Used for miscellaneous characters; it can vary by keyboard.
   * For the US standard keyboard, the ']}' key
   */
  BracketRight = 94,
  /**
   * Used for miscellaneous characters; it can vary by keyboard.
   * For the US standard keyboard, the ''"' key
   */
  Quote = 95,
  /**
   * Used for miscellaneous characters; it can vary by keyboard.
   */
  OEM_8 = 96,
  /**
   * Either the angle bracket key or the backslash key on the RT 102-key keyboard.
   */
  IntlBackslash = 97,
  Numpad0 = 98,// VK_NUMPAD0, 0x60, Numeric keypad 0 key
  Numpad1 = 99,// VK_NUMPAD1, 0x61, Numeric keypad 1 key
  Numpad2 = 100,// VK_NUMPAD2, 0x62, Numeric keypad 2 key
  Numpad3 = 101,// VK_NUMPAD3, 0x63, Numeric keypad 3 key
  Numpad4 = 102,// VK_NUMPAD4, 0x64, Numeric keypad 4 key
  Numpad5 = 103,// VK_NUMPAD5, 0x65, Numeric keypad 5 key
  Numpad6 = 104,// VK_NUMPAD6, 0x66, Numeric keypad 6 key
  Numpad7 = 105,// VK_NUMPAD7, 0x67, Numeric keypad 7 key
  Numpad8 = 106,// VK_NUMPAD8, 0x68, Numeric keypad 8 key
  Numpad9 = 107,// VK_NUMPAD9, 0x69, Numeric keypad 9 key
  NumpadMultiply = 108,// VK_MULTIPLY, 0x6A, Multiply key
  NumpadAdd = 109,// VK_ADD, 0x6B, Add key
  NUMPAD_SEPARATOR = 110,// VK_SEPARATOR, 0x6C, Separator key
  NumpadSubtract = 111,// VK_SUBTRACT, 0x6D, Subtract key
  NumpadDecimal = 112,// VK_DECIMAL, 0x6E, Decimal key
  NumpadDivide = 113,// VK_DIVIDE, 0x6F,
  /**
   * Cover all key codes when IME is processing input.
   */
  KEY_IN_COMPOSITION = 114,
  ABNT_C1 = 115,// Brazilian (ABNT) Keyboard
  ABNT_C2 = 116,// Brazilian (ABNT) Keyboard
  AudioVolumeMute = 117,
  AudioVolumeUp = 118,
  AudioVolumeDown = 119,
  BrowserSearch = 120,
  BrowserHome = 121,
  BrowserBack = 122,
  BrowserForward = 123,
  MediaTrackNext = 124,
  MediaTrackPrevious = 125,
  MediaStop = 126,
  MediaPlayPause = 127,
  LaunchMediaPlayer = 128,
  LaunchMail = 129,
  LaunchApp2 = 130,
  /**
   * VK_CLEAR, 0x0C, CLEAR key
   */
  Clear = 131,
  /**
   * Placed last to cover the length of the enum.
   * Please do not depend on this value!
   */
  MAX_VALUE = 132
}