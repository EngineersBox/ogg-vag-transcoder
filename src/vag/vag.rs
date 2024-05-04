pub const VAG_SAMPLE_BYTES: usize = 14;
pub const VAG_SAMPLE_NIBBLE: usize = VAG_SAMPLE_BYTES * 2;

pub enum VAGFlag {
    Nothing = 0,
    LoopLastBlock = 1,
    LoopRegion = 2,
    LoopEnd = 3,
    LoopFirstBlock = 4,
    Unk = 5,
    LoopStart = 6,
    PlaybackEnd = 7
}

#[derive(Default)]
pub struct VAGChunk {
    pub shift: i8,
    pub predict: i8,
    pub flags: u8,
    pub sample: [u8; VAG_SAMPLE_BYTES],
}
