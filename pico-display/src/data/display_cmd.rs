use core::cell::RefCell;
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use heapless::{Vec, String};
use epd_display::epd::three_color::ThreeColor;

const STRING_CAPACITY: usize = 80;
const MAX_BODY_LINES: usize = 10;

pub static SHARED_DISPLAY_CMD: Mutex<CriticalSectionRawMutex, RefCell<DisplayCmd>> = Mutex::new(RefCell::new(DisplayCmd::None));
pub static DISPLAY_CMD_READY: Channel<CriticalSectionRawMutex, (), 1> = Channel::new();

pub struct TextLine {
    text: String<STRING_CAPACITY>,
    color: ThreeColor
}

impl TextLine {
    pub fn new(text: &str, color: ThreeColor) -> Self {
        let text = Self::string_from(text).unwrap_or(String::new());
        TextLine { text, color }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn color(&self) -> ThreeColor {
        self.color
    }

    fn string_from<const CAPACITY: usize>(slice: &str) -> Option<String<CAPACITY>> {
        if slice.len() <= CAPACITY {
            String::try_from(slice).ok()
        } else {
            let last = slice.floor_char_boundary(CAPACITY);
            String::try_from(&slice[0..last]).ok()
        }
    }
}

pub struct TextPanelContent {
    title: TextLine,
    body: Vec<TextLine, MAX_BODY_LINES>,
}

impl TextPanelContent {
    pub fn new(title: TextLine) -> Self {
        TextPanelContent {
            title, body: Vec::new(),
        }
    }

    pub fn title(&self) -> &TextLine {
        &self.title
    }

    pub fn body_len(&self) -> usize {
        self.body.len()
    }

    pub fn body_line(&self, i: usize) -> Option<&TextLine> {
        self.body.get(i)
    }

    pub fn add_body_line(&mut self, body_line: TextLine) -> Result<(), ()>{
        self.body.push(body_line).or(Err(()))
    }
}

pub enum DisplayCmd {
    None,
    TextPanel(TextPanelContent)
}
