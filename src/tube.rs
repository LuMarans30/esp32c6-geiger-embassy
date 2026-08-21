pub const DEFAULT_DIVIDER: f32 = 153.8;

#[derive(Clone, Copy)]
pub struct TubePreset {
    pub name: &'static str,
    pub divider: f32,
}

pub const PRESETS: &[TubePreset] = &[
    TubePreset {
        name: "j305",
        divider: 153.8,
    },
    TubePreset {
        name: "j315",
        divider: 153.8,
    },
    TubePreset {
        name: "m4011",
        divider: 153.8,
    },
    TubePreset {
        name: "sbm20",
        divider: 175.0,
    },
    TubePreset {
        name: "si29bg",
        divider: 91.0,
    },
    TubePreset {
        name: "lnd712",
        divider: 108.0,
    },
    TubePreset {
        name: "lnd7317",
        divider: 65.0,
    },
    TubePreset {
        name: "sts5",
        divider: 116.0,
    },
    TubePreset {
        name: "sbt11a",
        divider: 318.0,
    },
];

pub fn find(name: &str) -> Option<TubePreset> {
    PRESETS
        .iter()
        .find(|preset| preset.name.eq_ignore_ascii_case(name))
        .copied()
}
