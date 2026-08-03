-------------------------------- MODULE Vidformer --------------------------------

\* TLA+ model of vidformer's rendering-engine concurrency: generation
\* planning, the decode pool (optimal eviction), decoder scheduling and
\* GOP abandonment, and the filter/encode output pipeline. Mirrors
\* vidformer/src/pool.rs and vidformer/src/dve.rs.
\*
\* Inv checks the core safety invariants; MyGoal checks termination.
\* To check (MC.tla / MC.cfg are in this directory):
\*   java -cp tla2tools.jar tlc2.TLC -workers auto -config MC.cfg MC.tla
\*
\* Note: Two decoders may implicitly merge in this model if they have the same frames left
\* That's not how real code would behave

EXTENDS Integers, Sequences, FiniteSets

CONSTANTS ScheduleLen, Frames, PoolSize, Gops, MaxDecoders, DecoderView
VARIABLES schedule, done_gens, next_gen, d_pool_ready, decoders, filtering, encode_buf, enc_next

vars == <<schedule, done_gens, next_gen, d_pool_ready, decoders, filtering, encode_buf, enc_next>>
pipeVars == <<filtering, encode_buf, enc_next>>

F_NOT_USED == 1000000

DecoderFramesLeft(d) == {f \in Frames : f[1] = d[1] /\ f[2] \in d[2]}

\* Return the next generation when a frame is needed, or F_NOT_USED if it's not
NextNeededGeneration(f) == IF \E g \in 1..ScheduleLen : g \notin done_gens /\ f \in schedule[g] THEN
                                LET
                                   used_gens == { g \in 1..ScheduleLen : g \notin done_gens /\ f \in schedule[g]}
                               IN
                                CHOOSE g \in used_gens : \A g2 \in used_gens : g <= g2
                           ELSE F_NOT_USED

DecoderUndeliveredFramesLeft(d) == {f \in DecoderFramesLeft(d) : f \notin d_pool_ready}

DecoderNextNeededGeneration(d) == IF DecoderUndeliveredFramesLeft(d) = {} THEN F_NOT_USED
                                  ELSE NextNeededGeneration(CHOOSE f \in DecoderUndeliveredFramesLeft(d) : \A of \in DecoderUndeliveredFramesLeft(d) : NextNeededGeneration(f) <= NextNeededGeneration(of))

NeedSet == {f \in Frames : \E g \in 1..ScheduleLen : g \notin done_gens /\ g < next_gen /\ f \in schedule[g]}

FutureSet == { f \in Frames : \E d \in decoders : f[1] = d[1] /\ f[2] \in d[2] }

ActiveGens == {g \in 1..ScheduleLen : g < next_gen /\ g \notin done_gens}

EvictionSet(n) == LET
                      evictable_frames == {f \in d_pool_ready : f \notin NeedSet /\ f \notin schedule[next_gen]}
                  IN
                      CHOOSE eset \in SUBSET evictable_frames : /\ Cardinality(eset) = n
                                                                /\ \A ef \in eset : \A kf \in (evictable_frames \ eset) : NextNeededGeneration(kf) <= NextNeededGeneration(ef)


Init == /\ schedule \in [1..ScheduleLen -> {a : a \in SUBSET Frames}] \* Find all possible schedules
        /\ \A i \in 1..ScheduleLen : Cardinality(schedule[i]) <= PoolSize \* Make sure it's possible to run the schedule
        /\ done_gens = {}
        /\ next_gen = 1
        /\ d_pool_ready = {}
        /\ decoders = {}
        /\ filtering = {}
        /\ encode_buf = {}
        /\ enc_next = 1

PlanGen == /\ next_gen <= ScheduleLen
           /\ Cardinality(ActiveGens) < DecoderView
           /\ Cardinality(NeedSet \cup schedule[next_gen]) <= PoolSize
           /\ d_pool_ready' = IF Cardinality(d_pool_ready \cup NeedSet \cup schedule[next_gen]) > PoolSize THEN
                                  \* We can plan the next gen, but we have too many leftover frames in the pool
                                  \* Remove enough d_pool_ready frames not in NeedSet to make room
                                  \* Keeps any frames which happen to be unused now but not in the new generation
                                  d_pool_ready \ EvictionSet(Cardinality(d_pool_ready \cup NeedSet \cup schedule[next_gen]) - PoolSize)
                              ELSE
                                  d_pool_ready
           /\ next_gen' = next_gen + 1
           /\ UNCHANGED<<done_gens, decoders, filtering, encode_buf, enc_next>>

CreateDecoder == /\ Cardinality(decoders) < MaxDecoders
                 /\ \E f \in NeedSet : /\ f \notin FutureSet
                                       /\ f \notin d_pool_ready
                 /\ LET
                        new_f == CHOOSE f \in NeedSet : /\ f \notin FutureSet
                                                        /\ f \notin d_pool_ready
                                                        \* Make sure that we are creating a decoder for the soonest-needed GOP
                                                        /\ \A other_f \in NeedSet : (other_f \notin FutureSet /\ other_f \notin d_pool_ready) => NextNeededGeneration(f) <= NextNeededGeneration(other_f)
                    IN
                        decoders' = decoders \union {<<new_f[1], {f[2] : f \in {f \in Frames : f[1] = new_f[1]}}>>}
                 /\ UNCHANGED<<done_gens, next_gen, d_pool_ready>> /\ UNCHANGED pipeVars

DecoderDecodeNeeded == \E d \in decoders : \E f \in NeedSet :       /\ f \notin d_pool_ready
                                                                    /\ f \in DecoderFramesLeft(d)
                                                                    /\ IF Cardinality(d_pool_ready) = PoolSize
                                                                       THEN LET cands == {p \in d_pool_ready : p \notin NeedSet}
                                                                            IN /\ cands # {}
                                                                               /\ LET victim == CHOOSE v \in cands : \A ov \in cands : NextNeededGeneration(v) >= NextNeededGeneration(ov)
                                                                                  IN d_pool_ready' = (d_pool_ready \ {victim}) \union {f}
                                                                       ELSE d_pool_ready' = d_pool_ready \union {f}
                                                                    /\ \/ /\ Cardinality(d[2]) > 1
                                                                          /\ decoders' = (decoders \ {d}) \union {<<d[1], {remaining_f \in d[2] : f[2] # remaining_f}>>}
                                                                       \/ /\ Cardinality(d[2]) = 1
                                                                          /\ decoders' = decoders \ {d}
                                                                    /\ UNCHANGED<<done_gens, next_gen>> /\ UNCHANGED pipeVars
                                                                    
DecoderDecodeNotNeeded == \E d \in decoders : \E f \in DecoderFramesLeft(d) :   /\ \E other_frame \in NeedSet : other_frame \notin d_pool_ready /\ other_frame \in DecoderFramesLeft(d) \* Make sure there is another frame in this decoder which we need to get to
                                                                                /\ f \in d_pool_ready \/ f \notin NeedSet
                                                                                /\ \/ /\ Cardinality(d[2]) > 1
                                                                                      /\ decoders' = (decoders \ {d}) \union {<<d[1], {remaining_f \in d[2] : f[2] # remaining_f}>>}
                                                                                   \/ /\ Cardinality(d[2]) = 1
                                                                                      /\ decoders' = decoders \ {d}
                                                                                /\ UNCHANGED<<done_gens, next_gen, d_pool_ready>> /\ UNCHANGED pipeVars

Stalled(d) == DecoderFramesLeft(d) \intersect (NeedSet \ d_pool_ready) = {}

DecoderDecodeOpportunistic ==
    \E d \in decoders : \E f \in DecoderFramesLeft(d) :
        /\ ~Stalled(d)
        /\ f \notin NeedSet
        /\ f \notin d_pool_ready
        /\ \/ /\ Cardinality(d_pool_ready) < PoolSize
              /\ d_pool_ready' = d_pool_ready \union {f}
           \/ /\ Cardinality(d_pool_ready) = PoolSize
              /\ LET cands == {p \in d_pool_ready : p \notin NeedSet}
                 IN /\ cands # {}
                    /\ LET victim == CHOOSE v \in cands : \A ov \in cands : NextNeededGeneration(v) >= NextNeededGeneration(ov)
                       IN /\ NextNeededGeneration(f) < NextNeededGeneration(victim)
                          /\ d_pool_ready' = (d_pool_ready \ {victim}) \union {f}
        /\ \/ /\ Cardinality(d[2]) > 1
              /\ decoders' = (decoders \ {d}) \union {<<d[1], {remaining_f \in d[2] : f[2] # remaining_f}>>}
           \/ /\ Cardinality(d[2]) = 1
              /\ decoders' = decoders \ {d}
        /\ UNCHANGED<<done_gens, next_gen>> /\ UNCHANGED pipeVars

\* Output pipeline (dve.rs): dispatch the lowest ready generation; filters
\* finish in any order; the encoder drains the buffer strictly in order.
FilterDispatch == \E g \in 1..ScheduleLen :
    /\ g < next_gen
    /\ g \notin done_gens
    /\ g \notin filtering
    /\ schedule[g] \subseteq d_pool_ready
    /\ \A g2 \in 1..(g-1) : g2 \in done_gens \/ g2 \in filtering \/ ~(schedule[g2] \subseteq d_pool_ready)
    /\ filtering' = filtering \union {g}
    /\ UNCHANGED<<done_gens, next_gen, d_pool_ready, decoders, encode_buf, enc_next>>

FilterFinish == \E g \in filtering :
    /\ filtering' = filtering \ {g}
    /\ done_gens' = done_gens \union {g}
    /\ encode_buf' = encode_buf \union {g}
    /\ UNCHANGED<<next_gen, d_pool_ready, decoders, enc_next>>

Encode == /\ enc_next \in encode_buf
          /\ encode_buf' = encode_buf \ {enc_next}
          /\ enc_next' = enc_next + 1
          /\ UNCHANGED<<done_gens, next_gen, d_pool_ready, decoders, filtering>>

DecoderAbandon == /\ Cardinality(decoders) = MaxDecoders
                  /\ \E d \in decoders : /\ \A od \in decoders : DecoderNextNeededGeneration(d) >= DecoderNextNeededGeneration(od) \* What's the least-soonest-used decoder?
                                         /\ \E f \in NeedSet : /\ f \notin FutureSet \* Are there frames that are not covered by existing decoders?
                                                               /\ f \notin d_pool_ready
                                                               /\ NextNeededGeneration(f) < DecoderNextNeededGeneration(d) \* Are we used after one of those frames?
                                         /\ DecoderFramesLeft(d) \intersect (NeedSet \ d_pool_ready) = {} \* Are we stalled?
                                         /\ decoders' = decoders \ {d}
                                         /\ UNCHANGED<<done_gens, next_gen, d_pool_ready>> /\ UNCHANGED pipeVars

IsEndState == /\ next_gen = ScheduleLen + 1
              /\ Cardinality(done_gens) = ScheduleLen
              /\ filtering = {}
              /\ encode_buf = {}
              /\ enc_next = ScheduleLen + 1

EndStutter == /\ IsEndState
              /\ UNCHANGED vars

Next == /\ \/ PlanGen
           \/ CreateDecoder
           \/ DecoderDecodeNeeded
           \/ DecoderDecodeNotNeeded
           \/ DecoderDecodeOpportunistic
           \/ FilterDispatch
           \/ FilterFinish
           \/ Encode
           \/ DecoderAbandon
           \/ EndStutter
        /\ UNCHANGED<<schedule>>



Inv == /\ \A d \in decoders : d[1] \in Gops
       /\ \A gop \in Gops : Cardinality({ d \in decoders : d[1] = gop}) < Cardinality({ f \in Frames : f[1] = gop}) + 1 \* Add one for a done gop
       /\ Cardinality(decoders) <= MaxDecoders
       /\ Cardinality(NeedSet) <= PoolSize
       /\ \A d \in decoders : Cardinality(d[2]) > 0
       /\ \A f \in NeedSet : NextNeededGeneration(f) # F_NOT_USED
       /\ Cardinality(d_pool_ready) <= PoolSize

\* Add weak fairness to everything or else we can't test liveness
Spec == Init /\ [][Next]_vars /\ WF_vars(Next)

MyGoal == []<>IsEndState

=============================================================================
