//! Three-way merge of `.pptx` files: two people edited one deck concurrently.
//!
//! Slides are the unit: edits to different slides, and added, removed or reordered slides, are
//! all kept; two different edits of one slide are a conflict. Deck-wide settings (masters, size,
//! sections) come from the side that changed them; media only the other side added is carried
//! over so no slide loses what it shows.

use std::sync::Arc;

use deckcraft_model::{LayoutId, MasterId, Presentation, Slide};
use storage_contracts::{MergeError, PackageMerger};

/// Merges `.pptx` files; see the module docs.
#[derive(Debug, Default, Clone, Copy)]
pub struct PptxMerger;

/// Layouts in file order across masters. Ids are handed out afresh on every import (they shift
/// with the slide count), so slides are compared by position in this list instead.
fn layout_ids(deck: &Presentation) -> Vec<LayoutId> {
    deck.masters
        .iter()
        .flat_map(|master| master.layouts.iter().map(|layout| layout.id))
        .collect()
}

fn position_of(deck: &Presentation, slide: &Slide) -> LayoutId {
    let index = layout_ids(deck)
        .iter()
        .position(|id| *id == slide.layout)
        .unwrap_or(usize::MAX);
    LayoutId(u32::try_from(index).unwrap_or(u32::MAX))
}

/// Slides with layout ids replaced by positions.
fn by_position(deck: &Presentation) -> Vec<Arc<Slide>> {
    deck.slides
        .iter()
        .map(|slide| {
            let mut slide = Slide::clone(slide);
            slide.layout = position_of(deck, &slide);
            Arc::new(slide)
        })
        .collect()
}

/// Deck-wide settings without slides, media and the ids that shift on import.
fn deck_settings(deck: &Presentation) -> Presentation {
    let mut rest = deck.clone();
    rest.slides = Vec::new();
    rest.media = Vec::new();
    rest.next_id = 0;
    for master in &mut rest.masters {
        let master = Arc::make_mut(master);
        master.id = MasterId::default();
        for layout in &mut master.layouts {
            layout.id = LayoutId::default();
        }
    }
    rest
}

impl PackageMerger for PptxMerger {
    fn handles(&self, name: &str) -> bool {
        name.rsplit_once('.')
            .is_some_and(|(_, ext)| ext.eq_ignore_ascii_case("pptx"))
    }

    fn merge(&self, base: &[u8], ours: &[u8], theirs: &[u8]) -> Result<Vec<u8>, MergeError> {
        let read = |bytes: &[u8]| deckcraft_pptx::import(bytes).map_err(|_| MergeError::Unreadable);
        let (base, ours, theirs) = (read(base)?, read(ours)?, read(theirs)?);
        let slides = merge3::merge3(
            &by_position(&base),
            &by_position(&ours),
            &by_position(&theirs),
        )
        .map_err(|_| MergeError::Conflict)?;
        let (base_rest, our_rest, their_rest) = (
            deck_settings(&base),
            deck_settings(&ours),
            deck_settings(&theirs),
        );
        let mut merged = if our_rest == their_rest || their_rest == base_rest {
            ours.clone()
        } else if our_rest == base_rest {
            theirs.clone()
        } else {
            return Err(MergeError::Conflict);
        };
        // Back from positions to the layout ids of the deck that is written.
        let ids = layout_ids(&merged);
        merged.slides = slides
            .into_iter()
            .map(|slide| {
                let mut slide = Slide::clone(&slide);
                if let Some(id) = ids.get(slide.layout.0 as usize) {
                    slide.layout = *id;
                }
                Arc::new(slide)
            })
            .collect();
        merged.media = ours.media;
        for item in theirs.media {
            if !merged.media.iter().any(|m| m.id == item.id) {
                merged.media.push(item);
            }
        }
        merged.next_id = ours.next_id.max(theirs.next_id);
        deckcraft_pptx::export(&merged).map_err(|_| MergeError::Unreadable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deck(names: &[&str]) -> Vec<u8> {
        let slides = names
            .iter()
            .enumerate()
            .map(|(index, name)| {
                Arc::new(Slide {
                    id: deckcraft_model::SlideId(u32::try_from(index).unwrap() + 1),
                    name: (*name).to_string(),
                    ..Slide::default()
                })
            })
            .collect();
        let deck = Presentation {
            slides,
            ..Presentation::default()
        };
        deckcraft_pptx::export(&deck).unwrap()
    }

    fn names(bytes: &[u8]) -> Vec<String> {
        deckcraft_pptx::import(bytes)
            .unwrap()
            .slides
            .iter()
            .map(|s| s.name.clone())
            .collect()
    }

    #[test]
    fn edits_to_different_slides_and_new_slides_are_kept() {
        let base = deck(&["intro", "plan"]);
        let ours = deck(&["intro v2", "plan"]);
        let theirs = deck(&["intro", "plan", "appendix"]);
        let merged = PptxMerger.merge(&base, &ours, &theirs).unwrap();
        assert_eq!(names(&merged), ["intro v2", "plan", "appendix"]);
    }

    #[test]
    fn one_slide_edited_two_ways_conflicts() {
        let base = deck(&["intro"]);
        assert_eq!(
            PptxMerger.merge(&base, &deck(&["a"]), &deck(&["b"])),
            Err(MergeError::Conflict)
        );
    }

    #[test]
    fn garbage_is_unreadable() {
        let base = deck(&["x"]);
        assert_eq!(
            PptxMerger.merge(&base, b"nope", &base),
            Err(MergeError::Unreadable)
        );
    }
}
