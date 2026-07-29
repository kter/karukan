//! Input state machine
//!
//! Defines the states of the IME and transitions between them.

use super::candidate::CandidateList;
use super::engine::Segment;
use super::preedit::Preedit;

/// The current state of the IME
#[derive(Debug, Clone, Default)]
pub enum InputState {
    /// No input, waiting for user to type
    #[default]
    Empty,

    /// Composing mode - building preedit text (hiragana, katakana, or alphabet)
    Composing {
        /// The preedit string being composed
        preedit: Preedit,
        /// Unconverted romaji buffer (e.g., "k" waiting for next char)
        romaji_buffer: String,
    },

    /// Conversion mode - selecting from candidates
    Conversion {
        /// The preedit string showing conversion result
        preedit: Preedit,
        /// Uncommitted conversion segments. The concatenation of every
        /// segment's reading always equals the engine input buffer text.
        segments: Vec<Segment>,
        /// Index of the focused segment (always in bounds for `segments`).
        focus: usize,
    },
}

impl InputState {
    /// Check if the engine is in the Empty (idle) state
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    /// Get the current preedit if any
    pub fn preedit(&self) -> Option<&Preedit> {
        match self {
            Self::Empty => None,
            Self::Composing { preedit, .. } => Some(preedit),
            Self::Conversion { preedit, .. } => Some(preedit),
        }
    }

    /// Get mutable reference to preedit
    pub fn preedit_mut(&mut self) -> Option<&mut Preedit> {
        match self {
            Self::Empty => None,
            Self::Composing { preedit, .. } => Some(preedit),
            Self::Conversion { preedit, .. } => Some(preedit),
        }
    }

    /// Get candidates in conversion state
    pub fn candidates(&self) -> Option<&CandidateList> {
        match self {
            Self::Conversion {
                segments, focus, ..
            } => segments.get(*focus).map(|segment| &segment.candidates),
            _ => None,
        }
    }

    /// Get mutable reference to candidates
    pub fn candidates_mut(&mut self) -> Option<&mut CandidateList> {
        match self {
            Self::Conversion {
                segments, focus, ..
            } => segments
                .get_mut(*focus)
                .map(|segment| &mut segment.candidates),
            _ => None,
        }
    }
}
