use ratatui::{
    style::palette::tailwind,
    symbols,
    widgets::{block::Title, Block, Padding},
};

pub fn bgcolor() -> tailwind::Palette {
    return tailwind::BLUE;
}

pub fn border_header<'a, T>(title: Option<T>) -> Block<'a>
where
    T: Into<Title<'a>> + 'a,
{
    let block = Block::bordered()
        .border_set(symbols::border::PLAIN)
        .border_style(bgcolor().c400)
        .padding(Padding::horizontal(1));
    if let Some(title) = title {
        block.title(title)
    } else {
        block
    }
}
