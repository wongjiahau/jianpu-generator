#![allow(clippy::indexing_slicing)]

use super::duration::parse_duration_suffixes;
use super::groups::{
    apply_closed_group_depth, apply_open_group_depth, validate_group_note_count, GroupStack,
    HasGroupDepth,
};
use super::timed_lexer::TimedLexToken;
use super::TimedUnitHead;
use crate::ast::parsed::ScoreEvent;
use crate::error::{JianPuError, Span, Spanned};

/// A thin wrapper over `Spanned<ScoreEvent>` that holds mutable group-depth fields so that the
/// generic `HasGroupDepth`-based helpers (`apply_closed_group_depth`, `apply_open_group_depth`)
/// can operate on them.  After depth is applied the wrapper is consumed into the final event list.
struct DepthEvent {
    spanned: Spanned<ScoreEvent>,
    group_membership: u8,
    group_continuation: u8,
}

impl DepthEvent {
    fn new(spanned: Spanned<ScoreEvent>) -> Self {
        Self {
            spanned,
            group_membership: 0,
            group_continuation: 0,
        }
    }

    /// Flush accumulated depth into the underlying `ScoreEvent` and return the `Spanned` value.
    fn into_spanned(mut self) -> Spanned<ScoreEvent> {
        apply_depth_to_event(
            &mut self.spanned.value,
            self.group_membership,
            self.group_continuation,
        );
        self.spanned
    }
}

impl HasGroupDepth for DepthEvent {
    fn group_membership(&self) -> u8 {
        self.group_membership
    }

    fn group_continuation(&self) -> u8 {
        self.group_continuation
    }

    fn set_group_membership(&mut self, value: u8) {
        self.group_membership = value;
    }

    fn set_group_continuation(&mut self, value: u8) {
        self.group_continuation = value;
    }
}

/// Push `group_membership` and `group_continuation` depth values into the event's inner struct
/// (only `Note` and `Chord` carry these fields; other variants are unaffected).
fn apply_depth_to_event(event: &mut ScoreEvent, membership: u8, continuation: u8) {
    match event {
        ScoreEvent::Note(n) => {
            n.group_membership = n.group_membership.saturating_add(membership);
            n.group_continuation = n.group_continuation.saturating_add(continuation);
            n.tie = n.group_continuation > 0;
        }
        ScoreEvent::Chord(c) => {
            c.group_membership = c.group_membership.saturating_add(membership);
            c.group_continuation = c.group_continuation.saturating_add(continuation);
            c.tie = c.group_continuation > 0;
        }
        ScoreEvent::Rest(r) => {
            r.group_membership = r.group_membership.saturating_add(membership);
            r.group_continuation = r.group_continuation.saturating_add(continuation);
        }
        _ => {}
    }
}

/// When a slur group's last element is an Extension (i.e., `)` follows a `-`), the arc should
/// end at the extension dash position rather than at the note head. This function scans the
/// group slice (after `apply_closed_group_depth` has run), finds such a pattern, and sets
/// `slur_group_close_at_duration` on the last Note/Chord in the group so the compiler can
/// close the arc at the right column.
fn annotate_slur_close_via_extension(group_slice: &mut [DepthEvent]) {
    // Check if the last element in the group is a closing Extension (continuation == 0).
    let last_is_closing_ext = group_slice
        .last()
        .map(|e| matches!(e.spanned.value, ScoreEvent::Extension) && e.group_continuation == 0)
        .unwrap_or(false);

    if !last_is_closing_ext {
        return;
    }

    // Find the last Note or Chord in the group slice — this is the note being extended.
    let last_note_idx = group_slice
        .iter()
        .rposition(|e| matches!(e.spanned.value, ScoreEvent::Note(_) | ScoreEvent::Chord(_)));

    let Some(note_idx) = last_note_idx else {
        return;
    };

    // Count Extension events with continuation > 0 that appear after the note — these are
    // the "continuing" extensions that precede the final closing extension.
    let num_continuing_exts = group_slice[note_idx + 1..]
        .iter()
        .filter(|e| matches!(e.spanned.value, ScoreEvent::Extension) && e.group_continuation > 0)
        .count() as u32;

    let note_initial_duration = match &group_slice[note_idx].spanned.value {
        ScoreEvent::Note(n) => n.duration,
        ScoreEvent::Chord(c) => c.duration,
        _ => return,
    };

    // close_offset = position of the last extension dash relative to the note's start col.
    let close_offset = note_initial_duration + num_continuing_exts * 4;

    match &mut group_slice[note_idx].spanned.value {
        ScoreEvent::Note(n) => n.slur_group_close_at_duration = Some(close_offset),
        ScoreEvent::Chord(c) => c.slur_group_close_at_duration = Some(close_offset),
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

pub struct TimedRdParser<'a, H: TimedUnitHead> {
    source: &'a str,
    base_offset: usize,
    tokens: &'a [Spanned<TimedLexToken>],
    pos: usize,
    stack: &'a mut GroupStack,
    /// Staging area: events with their pending depth accumulators.
    staging: Vec<DepthEvent>,
    _head: std::marker::PhantomData<H>,
}

impl<'a, H: TimedUnitHead> TimedRdParser<'a, H> {
    pub fn parse_line(
        source: &'a str,
        base_offset: usize,
        tokens: &'a [Spanned<TimedLexToken>],
        stack: &'a mut GroupStack,
    ) -> Result<Vec<Spanned<ScoreEvent>>, JianPuError> {
        // Frames carried over from a previous bar have segment_start values that
        // refer to the old staging vec.  Reset them to 0 so they cover all events
        // produced in this new call.
        for frame in stack.frames.iter_mut() {
            frame.segment_start = 0;
        }

        let mut parser = Self {
            source,
            base_offset,
            tokens,
            pos: 0,
            stack,
            staging: Vec::new(),
            _head: std::marker::PhantomData,
        };
        parser.parse_atoms(false)?;
        parser.finalize_open_frames()?;
        let events = parser
            .staging
            .into_iter()
            .map(|d| d.into_spanned())
            .collect();
        Ok(events)
    }

    // -----------------------------------------------------------------------
    // Token stream helpers
    // -----------------------------------------------------------------------

    fn peek(&self) -> Option<&TimedLexToken> {
        self.tokens.get(self.pos).map(|s| &s.value)
    }

    fn peek_span(&self) -> Option<&Span> {
        self.tokens.get(self.pos).map(|s| &s.span)
    }

    fn bump(&mut self) -> Option<&Spanned<TimedLexToken>> {
        let tok = self.tokens.get(self.pos);
        if tok.is_some() {
            self.pos += 1;
        }
        tok
    }

    fn current_span(&self) -> Span {
        self.peek_span()
            .cloned()
            .unwrap_or_else(|| Span::new(self.base_offset, self.base_offset))
    }

    // -----------------------------------------------------------------------
    // Core recursive methods
    // -----------------------------------------------------------------------

    fn parse_atoms(&mut self, stop_at_rparen: bool) -> Result<(), JianPuError> {
        loop {
            match self.peek() {
                None => return Ok(()),
                Some(TimedLexToken::RParen) => {
                    if stop_at_rparen {
                        return Ok(());
                    }
                    self.close_group()?;
                }
                Some(TimedLexToken::LParen) => {
                    self.open_group()?;
                }
                Some(TimedLexToken::Extension) => {
                    let span = self.current_span();
                    self.bump();
                    self.staging
                        .push(DepthEvent::new(Spanned::new(ScoreEvent::Extension, span)));
                }
                Some(TimedLexToken::HeadStart { offset }) => {
                    let offset = *offset;
                    self.parse_timed_unit(offset)?;
                }
                Some(TimedLexToken::Bpm(bpm)) => {
                    let bpm = *bpm;
                    let span = self.current_span();
                    self.bump();
                    self.staging.push(DepthEvent::new(Spanned::new(
                        ScoreEvent::BpmChange(bpm),
                        span,
                    )));
                }
                Some(TimedLexToken::KeyChange(key)) => {
                    let key = key.clone();
                    let span = self.current_span();
                    self.bump();
                    self.staging.push(DepthEvent::new(Spanned::new(
                        ScoreEvent::KeyChange(key),
                        span,
                    )));
                }
                Some(TimedLexToken::TimeSignature { num, den }) => {
                    let numerator = *num;
                    let denominator = *den;
                    let span = self.current_span();
                    self.bump();
                    self.staging.push(DepthEvent::new(Spanned::new(
                        ScoreEvent::TimeSignatureChange {
                            numerator,
                            denominator,
                        },
                        span,
                    )));
                }
            }
        }
    }

    /// Parse one timed unit (note/rest/chord head + duration suffixes) starting at `digit_offset`
    /// (which is an absolute byte offset into `self.source`).
    fn parse_timed_unit(&mut self, digit_offset: usize) -> Result<(), JianPuError> {
        // Relative byte offset from the start of `source`.
        let rel = digit_offset - self.base_offset;

        // Slice from the head offset to the end of the current non-whitespace word.
        // Duration suffixes are never whitespace, so the unit ends at the first whitespace char.
        let raw_text = &self.source[rel..];
        let text = raw_text
            .find(|c: char| c.is_whitespace() || c == '|' || c == '(' || c == ')')
            .map(|ws_pos| &raw_text[..ws_pos])
            .unwrap_or(raw_text);

        let chars: Vec<char> = text.chars().collect();

        // Build a span that covers the single head character (will be refined after suffixes).
        let head_span = Span::new(digit_offset, digit_offset + 1);

        // Parse the head (note digit / chord symbol).
        let (head, head_end, is_rest) = H::parse_head(&chars, 0, &head_span)?;

        // Parse duration suffixes.
        let duration_meta = parse_duration_suffixes::<H>(&chars, 0, head_end, is_rest, &head_span)?;

        let octave = if duration_meta.octave_up > 0 {
            duration_meta.octave_up
        } else {
            -duration_meta.octave_down
        };

        // Calculate the actual byte length covered by this unit.
        let unit_byte_len: usize = chars[..duration_meta.next_index]
            .iter()
            .map(|c| c.len_utf8())
            .sum();
        let unit_end_abs = digit_offset + unit_byte_len;
        let unit_span = Span::new(digit_offset, unit_end_abs);

        let event = H::to_event(
            &head,
            duration_meta.duration,
            duration_meta.dotted,
            octave,
            0,
            0,
        );
        self.staging
            .push(DepthEvent::new(Spanned::new(event, unit_span)));

        // Increment note count in the innermost open group frame.
        self.stack.increment_note_count();

        // Consume the HeadStart token for this unit.
        self.bump();

        // Skip any HeadStart tokens that fall within the byte range of the unit we just parsed.
        // This happens when the lexer emits a HeadStart for a digit inside a multi-char symbol
        // (e.g. the `7` in chord `1m7`).
        while let Some(TimedLexToken::HeadStart { offset }) = self.peek() {
            if *offset < unit_end_abs {
                self.bump();
            } else {
                break;
            }
        }

        Ok(())
    }

    /// Handle `(` — push a new frame and recurse into the inner atom sequence.
    fn open_group(&mut self) -> Result<(), JianPuError> {
        let lparen_span = self.current_span();
        self.bump(); // consume LParen

        let segment_start = self.staging.len();
        self.stack.push(segment_start);

        // Parse inner atoms until `)` or end of token stream.
        self.parse_atoms(true)?;

        // Now we should see `)` or end of stream.
        match self.peek() {
            Some(TimedLexToken::RParen) => {
                // Closed group — consume and apply closed-group depth.
                let rparen_span = self.current_span();
                self.bump();

                let frame = self.stack.pop().ok_or_else(|| {
                    JianPuError::new(
                        rparen_span.clone(),
                        "unexpected `)` — no open group".to_string(),
                    )
                })?;

                let note_count = frame.note_count;
                validate_group_note_count(note_count, &rparen_span)?;
                apply_closed_group_depth(&mut self.staging[frame.segment_start..]);
            }
            _ => {
                // No closing paren — treat as an open (cross-line) group: apply open depth.
                // The frame stays on the stack for `finalize_open_frames`.
            }
        }

        let _ = lparen_span; // suppress unused warning
        Ok(())
    }

    /// Handle `)` when encountered outside of `parse_atoms(stop_at_rparen=true)`.
    /// This closes the innermost frame that was left open from a previous call.
    fn close_group(&mut self) -> Result<(), JianPuError> {
        let rparen_span = self.current_span();
        self.bump(); // consume RParen

        let frame = self.stack.pop().ok_or_else(|| {
            JianPuError::new(
                rparen_span.clone(),
                "unexpected `)` — no open group".to_string(),
            )
        })?;

        let note_count = frame.note_count;
        validate_group_note_count(note_count, &rparen_span)?;
        apply_closed_group_depth(&mut self.staging[frame.segment_start..]);
        annotate_slur_close_via_extension(&mut self.staging[frame.segment_start..]);

        Ok(())
    }

    /// At end-of-line, any frames still on the stack represent open (cross-line) groups.
    /// Apply open-group depth to the events that belong to those frames.
    fn finalize_open_frames(&mut self) -> Result<(), JianPuError> {
        // Iterate from outermost to innermost (bottom of stack to top).
        for frame in &self.stack.frames {
            apply_open_group_depth(&mut self.staging[frame.segment_start..]);
        }
        Ok(())
    }
}
