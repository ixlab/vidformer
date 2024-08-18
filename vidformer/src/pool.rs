use crate::dve::IFrameRef;
use crate::dve::{AVFrame, Config, Context, SourceRef};
use std::fmt::Debug;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

const F_NOT_USED: usize = usize::MAX;

pub(crate) struct Pool {
    done_gens_recent: BTreeSet<usize>,
    done_gens_past: usize, // If a generation is less than this it is done
    next_gen: usize,
    members: BTreeMap<IFrameRef, Arc<AVFrame>>,
    pub(crate) decoders: BTreeMap<String, crate::dve::DecoderState>,
    pub(crate) finished_unjoined_decoders: BTreeSet<String>,
    pub(crate) terminate_decoders: bool,

    iframes_per_oframe: Vec<BTreeSet<IFrameRef>>,
    iframe_refs_in_out_idx: BTreeMap<IFrameRef, BTreeSet<usize>>,
    dve_context: Arc<Context>,
    dve_config: Arc<Config>,
}

impl Pool {
    pub(crate) fn new(
        iframes_per_oframe: Vec<BTreeSet<IFrameRef>>,
        iframe_refs_in_out_idx: BTreeMap<IFrameRef, BTreeSet<usize>>,
        dve_context: Arc<Context>,
        dve_config: Arc<Config>,
    ) -> Result<Self, crate::Error> {
        if dve_config.decode_pool_size == 0 {
            return Err(crate::Error::ConfigError(
                "decode_pool_size must be greater than 0".to_string(),
            ));
        }

        let mut out = Pool {
            done_gens_recent: BTreeSet::new(),
            done_gens_past: 0,
            next_gen: 0,
            members: BTreeMap::new(),
            decoders: BTreeMap::new(),
            finished_unjoined_decoders: BTreeSet::new(),
            terminate_decoders: false,
            iframes_per_oframe,
            iframe_refs_in_out_idx,
            dve_context,
            dve_config,
        };
        while out.plan_gen() {}
        Ok(out)
    }

    fn next_needed_gen(&self, frame: &IFrameRef) -> usize {
        let frame_uses = match self.iframe_refs_in_out_idx.get(frame) {
            Some(uses) => uses,
            None => return F_NOT_USED,
        };
        // TODO: We could throw a binary search in here to speed up cases where there are many uses of a frame, such as a watermark image
        for frame_use_gen in frame_uses {
            if frame_use_gen >= &self.done_gens_past
                && !self.done_gens_recent.contains(frame_use_gen)
            {
                return *frame_use_gen;
            }
        }
        F_NOT_USED
    }

    fn decoder_next_needed_gen(&self, decoder_id: &str) -> usize {
        let decoder = self.decoders.get(decoder_id).unwrap();
        let mut next_needed_gen = F_NOT_USED;
        for frame in &decoder.future_frames {
            let frame_next_needed = self.next_needed_gen(&IFrameRef {
                sourceref: decoder.source.clone(),
                pts: *frame,
            });
            if frame_next_needed < next_needed_gen {
                next_needed_gen = frame_next_needed;
            }
        }
        next_needed_gen
    }

    fn frame_gop(&self, frame: &IFrameRef) -> usize {
        let source = self.dve_context.sources.get(&frame.sourceref).unwrap();
        debug_assert!(source.ts.binary_search(&frame.pts).is_ok());
        match source.keys.binary_search(&frame.pts) {
            Ok(i) => i,
            Err(i) => i - 1, // i is the index of the first element greater than frame.pts
        }
    }

    pub(crate) fn new_decoder_gop(&self) -> Option<(SourceRef, usize)> {
        debug_assert!(self.decoders.len() <= self.dve_config.decoders);
        if self.decoders.len() == self.dve_config.decoders {
            return None;
        }
        let need_set = self
            .need_set()
            .into_iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let future_set: BTreeSet<IFrameRef> = self.future_set();
        let basis_frames: BTreeSet<IFrameRef> = need_set
            .difference(&self.members.keys().cloned().collect())
            .cloned()
            .collect::<BTreeSet<_>>()
            .difference(&future_set)
            .cloned()
            .collect();

        if !basis_frames.is_empty() {
            let mut iter = basis_frames.iter();
            let mut soonest_needed_basis_frame = iter.next().unwrap();
            let mut soonest_needed_basis_frame_next_needed =
                self.next_needed_gen(soonest_needed_basis_frame);

            for frame in iter {
                let candidate_frame_next_needed = self.next_needed_gen(frame);
                if candidate_frame_next_needed < soonest_needed_basis_frame_next_needed {
                    soonest_needed_basis_frame = frame;
                    soonest_needed_basis_frame_next_needed = candidate_frame_next_needed;
                }
            }

            let gop_id = self.frame_gop(soonest_needed_basis_frame);
            Some((soonest_needed_basis_frame.sourceref.clone(), gop_id))
        } else {
            None
        }
    }

    fn eviction_set(
        &self,
        size: usize,
        next_need_set: &BTreeSet<IFrameRef>,
    ) -> BTreeSet<IFrameRef> {
        struct FrameEvictionCandidate<'b> {
            needed_gen: usize,
            frame_ts: &'b IFrameRef,
        }
        impl<'b> Ord for FrameEvictionCandidate<'b> {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                self.needed_gen.cmp(&other.needed_gen)
            }
        }
        impl<'b> PartialOrd for FrameEvictionCandidate<'b> {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }
        impl<'b> Eq for FrameEvictionCandidate<'b> {}
        impl<'b> PartialEq for FrameEvictionCandidate<'b> {
            fn eq(&self, other: &Self) -> bool {
                self.needed_gen == other.needed_gen
            }
        }

        let mut heap = std::collections::BinaryHeap::new();
        for frame_ts in self.members.keys() {
            if !next_need_set.contains(frame_ts) {
                heap.push(FrameEvictionCandidate {
                    needed_gen: self.next_needed_gen(frame_ts),
                    frame_ts,
                });
            }
        }

        debug_assert!(heap.len() >= size);
        let mut out = BTreeSet::new();
        for _ in 0..size {
            let evicted = heap.pop().unwrap();
            out.insert(evicted.frame_ts.clone());
        }
        out
    }

    pub(crate) fn should_stall(&self, decoder_id: &str) -> bool {
        // Is there a frame in the need set which is not in the members set but is in our future set?
        // If not, stall

        let decoder = self.decoders.get(decoder_id).unwrap();

        let need_set = self
            .need_set()
            .into_iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let dec_future_set = decoder.future_iframerefs();
        let members_set = self.members.keys().cloned().collect::<BTreeSet<_>>();

        let missing_frames = need_set
            .difference(&members_set)
            .cloned()
            .collect::<BTreeSet<_>>();

        missing_frames.intersection(&dec_future_set).count() == 0
    }

    pub(crate) fn decoded(&mut self, decoder_id: &str, frame: IFrameRef, avframe: Arc<AVFrame>) {
        debug_assert!(self.members.len() <= self.dve_config.decode_pool_size);
        debug_assert!(!self.should_stall(decoder_id));

        if self.members.contains_key(&frame) {
            // Frame already present
            return;
        }

        let need_set: BTreeSet<IFrameRef> = self.need_set().into_iter().cloned().collect();

        if need_set.contains(&frame) || self.members.len() < self.dve_config.decode_pool_size {
            // If the pool is full evict a cache frame
            if self.members.len() == self.dve_config.decode_pool_size {
                let evict_set = self.eviction_set(1, &need_set);
                debug_assert_eq!(evict_set.len(), 1);
                for frame_ts in evict_set {
                    self.members.remove(&frame_ts);
                }
            }
            self.members.insert(frame, avframe);
        } else {
            // See if we can evict a frame to make room
            let f_next_need = self.next_needed_gen(&frame);
            if f_next_need < F_NOT_USED {
                let mut least_needed_pool_frame: Option<IFrameRef> = None;
                let mut least_needed_pool_frame_next_needed = F_NOT_USED;

                for pool_frame in self.members.keys() {
                    if !need_set.contains(pool_frame) {
                        if least_needed_pool_frame.is_some() {
                            let pool_frame_next_needed = self.next_needed_gen(pool_frame);
                            if pool_frame_next_needed > least_needed_pool_frame_next_needed {
                                least_needed_pool_frame = Some(pool_frame.clone());
                                least_needed_pool_frame_next_needed = pool_frame_next_needed;
                            }
                        } else {
                            least_needed_pool_frame = Some(pool_frame.clone());
                            least_needed_pool_frame_next_needed = self.next_needed_gen(pool_frame);
                        }
                    }
                }

                if f_next_need < least_needed_pool_frame_next_needed {
                    log::info!("Evicting frame {:?} (needed in gen {}) for sooner-needed frame {:?} (needed in gen {})", least_needed_pool_frame, least_needed_pool_frame_next_needed, frame, f_next_need);
                    self.members.remove(&least_needed_pool_frame.unwrap());
                    self.members.insert(frame, avframe);
                }
            }
        }

        debug_assert!(self.members.len() <= self.dve_config.decode_pool_size);
    }

    pub(crate) fn should_decoder_abandon(&self, decoder_id: &str) -> bool {
        if self.decoders.len() < self.dve_config.decoders || !self.should_stall(decoder_id) {
            return false;
        }

        let dec_next_needed_gen = self.decoder_next_needed_gen(decoder_id);

        // check that there is a frame we should create a new decoder for
        let mut found_sooner_basis_frame = false;
        let future_set = self.future_set();
        for frame in self.need_set() {
            if self.members.contains_key(frame) || future_set.contains(frame) {
                continue;
            }
            let frame_next_needed_gen = self.next_needed_gen(frame);
            if frame_next_needed_gen < dec_next_needed_gen {
                found_sooner_basis_frame = true;
                break;
            }
        }
        if !found_sooner_basis_frame {
            return false;
        }

        // check if all other decoders have a lower or equal next needed gen
        // aka make sure this is the least-soonest-needed decoder
        for other_decoder_id in self.decoders.keys() {
            if other_decoder_id == decoder_id {
                continue;
            }
            let other_dec_next_needed_gen = self.decoder_next_needed_gen(other_decoder_id);
            if other_dec_next_needed_gen > dec_next_needed_gen {
                return false;
            }
        }

        true
    }

    pub(crate) fn future_set(&self) -> BTreeSet<IFrameRef> {
        let mut out = BTreeSet::new();
        for decoder in self.decoders.values() {
            out.extend(decoder.future_frames.iter().map(|t| IFrameRef {
                sourceref: decoder.source.clone(),
                pts: *t,
            }));
        }
        out
    }

    fn plan_gen(&mut self) -> bool {
        debug_assert!(self.next_gen <= self.iframes_per_oframe.len());
        if self.next_gen == self.iframes_per_oframe.len() {
            return false;
        }

        let number_of_active_gens =
            self.next_gen - self.done_gens_past - self.done_gens_recent.len();
        debug_assert_eq!(number_of_active_gens, self.active_gens().len());
        if number_of_active_gens >= self.dve_config.decoder_view {
            return false;
        }

        let mut next_need_set: BTreeSet<IFrameRef> =
            { self.need_set().iter().map(|a| (*a).clone()).collect() };
        next_need_set.extend(
            self.iframes_per_oframe[self.next_gen]
                .iter()
                .map(|a| (*a).clone()),
        );

        if next_need_set.len() > self.dve_config.decode_pool_size {
            // not enough space in the pool even with evictions
            return false;
        }

        // Enough space, but check if we need to evict some frames first
        let current_members = self.members.keys().map(|a| (*a).clone()).collect();
        let next_need_set_union_current_members: BTreeSet<_> =
            next_need_set.union(&current_members).cloned().collect();
        if next_need_set_union_current_members.len() > self.dve_config.decode_pool_size {
            let needed_evictions =
                next_need_set_union_current_members.len() - self.dve_config.decode_pool_size;
            let evict_set = self.eviction_set(needed_evictions, &next_need_set);
            debug_assert!(evict_set.len() == needed_evictions);
            for frame_ts in evict_set {
                debug_assert!(!next_need_set.contains(&frame_ts));
                self.members.remove(&frame_ts);
            }
        }
        self.next_gen += 1;
        true
    }

    pub(crate) fn active_gens(&self) -> BTreeSet<usize> {
        let mut out = BTreeSet::new();
        for g in self.done_gens_past..self.next_gen {
            if !self.done_gens_recent.contains(&g) {
                out.insert(g);
            }
        }
        out
    }

    pub(crate) fn finish_gen(&mut self, gen: usize) {
        debug_assert!(self.active_gens().contains(&gen));

        // update the done_gens
        self.done_gens_recent.insert(gen);
        loop {
            match self.done_gens_recent.first() {
                Some(&first) if first == self.done_gens_past => {
                    self.done_gens_recent.remove(&first);
                    self.done_gens_past += 1;
                }
                _ => break,
            }
        }

        // plan future gens
        while self.plan_gen() {}
    }

    pub(crate) fn is_gen_ready(&self, gen: usize) -> bool {
        debug_assert!(self.active_gens().contains(&gen));

        self.iframes_per_oframe[gen]
            .iter()
            .all(|dep_frame| self.members.contains_key(dep_frame))
    }

    pub(crate) fn get_ready_gen_frames(&self, gen: usize) -> BTreeMap<IFrameRef, Arc<AVFrame>> {
        debug_assert!(self.is_gen_ready(gen));
        self.iframes_per_oframe[gen]
            .iter()
            .map(|iframeref| (iframeref.clone(), self.members[iframeref].clone()))
            .collect()
    }

    pub(crate) fn need_set(&self) -> BTreeSet<&IFrameRef> {
        let mut out = BTreeSet::new();
        for gen in self.active_gens() {
            out.extend(self.iframes_per_oframe[gen].iter());
        }
        out
    }
}

impl Debug for Pool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pool")
            .field("done_gens_recent", &self.done_gens_recent)
            .field("done_gens_past", &self.done_gens_past)
            .field("next_gen", &self.next_gen)
            .field("members", &self.members.keys().collect::<Vec<_>>())
            .field("decoders", &self.decoders.keys().collect::<Vec<_>>())
            .finish()
    }
}
