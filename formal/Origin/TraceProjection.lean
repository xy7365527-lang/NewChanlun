/-
Origin/TraceProjection.lean

Generic projection theorem for deterministic trace engines.

If a state projection carries both transition and observation, then equal
projected seeds imply equal traces for every finite future event list.  This is
the Lean shape behind the Rust `OrderTraceSeedKey` audit: sufficient seed, not
minimal quotient.
-/

import Origin.FiniteTraceQuotient

namespace NewChanlun.Origin

universe u v w z a

/-- Trace produced by stepping first, then observing the post-step state. -/
def runTrace {State : Type u} {Event : Type v} {Out : Type w}
    (step : State -> Event -> State) (observe : State -> Out) :
    State -> List Event -> List Out
  | _state, [] => []
  | state, event :: rest =>
      let next := step state event
      observe next :: runTrace step observe next rest

/-- Seed-level version of `runTrace`. -/
def runSeedTrace {Seed : Type z} {Event : Type v} {Out : Type w}
    (stepSeed : Seed -> Event -> Seed) (observeSeed : Seed -> Out) :
    Seed -> List Event -> List Out
  | _seed, [] => []
  | seed, event :: rest =>
      let next := stepSeed seed event
      observeSeed next :: runSeedTrace stepSeed observeSeed next rest

/--
  If `project` carries transition and observation, concrete traces factor
  through the seed trace for every finite future.
-/
theorem run_trace_factors_through_seed
    {State : Type u} {Event : Type v} {Seed : Type z} {Out : Type w}
    (step : State -> Event -> State) (observe : State -> Out)
    (project : State -> Seed)
    (stepSeed : Seed -> Event -> Seed) (observeSeed : Seed -> Out)
    (hstep : ∀ state event, project (step state event) = stepSeed (project state) event)
    (hobserve : ∀ state, observe state = observeSeed (project state))
    (state : State) (future : List Event) :
    runTrace step observe state future =
      runSeedTrace stepSeed observeSeed (project state) future := by
  induction future generalizing state with
  | nil =>
      rfl
  | cons event rest ih =>
      simp [runTrace, runSeedTrace, hobserve, hstep, ih]

/-- Equal projected seeds imply equal concrete traces for every finite future. -/
theorem same_seed_same_future_trace
    {State : Type u} {Event : Type v} {Seed : Type z} {Out : Type w}
    (step : State -> Event -> State) (observe : State -> Out)
    (project : State -> Seed)
    (stepSeed : Seed -> Event -> Seed) (observeSeed : Seed -> Out)
    (hstep : ∀ state event, project (step state event) = stepSeed (project state) event)
    (hobserve : ∀ state, observe state = observeSeed (project state))
    {left right : State}
    (hseed : project left = project right)
    (future : List Event) :
    runTrace step observe left future = runTrace step observe right future := by
  rw [run_trace_factors_through_seed step observe project stepSeed observeSeed hstep hobserve,
    run_trace_factors_through_seed step observe project stepSeed observeSeed hstep hobserve,
    hseed]

/-- Equal projected seeds imply equal finite-corpus trace classes. -/
theorem same_seed_same_finite_corpus_class
    {State : Type u} {Event : Type v} {Seed : Type z} {Out : Type w}
    (step : State -> Event -> State) (observe : State -> Out)
    (project : State -> Seed)
    (stepSeed : Seed -> Event -> Seed) (observeSeed : Seed -> Out)
    (hstep : ∀ state event, project (step state event) = stepSeed (project state) event)
    (hobserve : ∀ state, observe state = observeSeed (project state))
    {left right : State}
    (hseed : project left = project right)
    (corpus : List (List Event)) :
    finiteCorpusClassify (runTrace step observe) corpus left =
      finiteCorpusClassify (runTrace step observe) corpus right := by
  apply finite_corpus_behavior_same_class (trace := runTrace step observe)
  intro future _hfuture
  exact same_seed_same_future_trace step observe project stepSeed observeSeed hstep hobserve
    hseed future

/-- Output trace for Mealy-style engines: each event emits an output while stepping. -/
def runOutputTrace {State : Type u} {Event : Type v} {Out : Type w}
    (step : State -> Event -> State) (output : State -> Event -> Out) :
    State -> List Event -> List Out
  | _state, [] => []
  | state, event :: rest =>
      output state event :: runOutputTrace step output (step state event) rest

/-- Seed-level Mealy output trace. -/
def runSeedOutputTrace {Seed : Type z} {Event : Type v} {Out : Type w}
    (stepSeed : Seed -> Event -> Seed) (outputSeed : Seed -> Event -> Out) :
    Seed -> List Event -> List Out
  | _seed, [] => []
  | seed, event :: rest =>
      outputSeed seed event :: runSeedOutputTrace stepSeed outputSeed (stepSeed seed event) rest

/--
  Mealy-style version: if projected transition and event output both factor
  through the seed, concrete output traces factor through seed output traces.
-/
theorem run_output_trace_factors_through_seed
    {State : Type u} {Event : Type v} {Seed : Type z} {Out : Type w}
    (step : State -> Event -> State) (output : State -> Event -> Out)
    (project : State -> Seed)
    (stepSeed : Seed -> Event -> Seed) (outputSeed : Seed -> Event -> Out)
    (hstep : ∀ state event, project (step state event) = stepSeed (project state) event)
    (houtput : ∀ state event, output state event = outputSeed (project state) event)
    (state : State) (future : List Event) :
    runOutputTrace step output state future =
      runSeedOutputTrace stepSeed outputSeed (project state) future := by
  induction future generalizing state with
  | nil =>
      rfl
  | cons event rest ih =>
      simp [runOutputTrace, runSeedOutputTrace, houtput, hstep, ih]

/-- Equal projected seeds imply equal Mealy output traces for every finite future. -/
theorem same_seed_same_future_output_trace
    {State : Type u} {Event : Type v} {Seed : Type z} {Out : Type w}
    (step : State -> Event -> State) (output : State -> Event -> Out)
    (project : State -> Seed)
    (stepSeed : Seed -> Event -> Seed) (outputSeed : Seed -> Event -> Out)
    (hstep : ∀ state event, project (step state event) = stepSeed (project state) event)
    (houtput : ∀ state event, output state event = outputSeed (project state) event)
    {left right : State}
    (hseed : project left = project right)
    (future : List Event) :
    runOutputTrace step output left future = runOutputTrace step output right future := by
  rw [run_output_trace_factors_through_seed step output project stepSeed outputSeed hstep houtput,
    run_output_trace_factors_through_seed step output project stepSeed outputSeed hstep houtput,
    hseed]

/-- Equal projected seeds imply equal Mealy finite-corpus output classes. -/
theorem same_seed_same_output_corpus_class
    {State : Type u} {Event : Type v} {Seed : Type z} {Out : Type w}
    (step : State -> Event -> State) (output : State -> Event -> Out)
    (project : State -> Seed)
    (stepSeed : Seed -> Event -> Seed) (outputSeed : Seed -> Event -> Out)
    (hstep : ∀ state event, project (step state event) = stepSeed (project state) event)
    (houtput : ∀ state event, output state event = outputSeed (project state) event)
    {left right : State}
    (hseed : project left = project right)
    (corpus : List (List Event)) :
    finiteCorpusClassify (runOutputTrace step output) corpus left =
      finiteCorpusClassify (runOutputTrace step output) corpus right := by
  apply finite_corpus_behavior_same_class (trace := runOutputTrace step output)
  intro future _hfuture
  exact same_seed_same_future_output_trace step output project stepSeed outputSeed hstep houtput
    hseed future

/--
  L0 target contract only: an order/action observation is the intended default
  `TraceOut` shape for order/action trace output.  This is not a Rust
  CompleteClass realization, and runtime audits still falsify the full-state
  quotient.  The boundary/flip condition is represented by the chosen event
  future; the anti-degeneration witness below shows this shape can separate
  two classes.
-/
structure OrderActionObservation (Order : Type u) (Action : Type v) where
  order : Order
  action : Action
deriving DecidableEq, Repr

/--
  L0 finite trace alias for order/action observations.  This target contract is
  deliberately generic: boundary/flip futures are supplied by the caller, and
  the anti-degeneration witness below is only a tiny toy separation.
-/
def OrderActionFiniteTrace (Order : Type w) (Action : Type z) :
    Type (max w z) :=
  List (OrderActionObservation Order Action)

/--
  L0 order/action trace output, built from the generic Mealy-style trace.  This
  is the intended default `TraceOut` shape, not a Rust CompleteClass
  realization and not a full-state quotient claim; boundary/flip condition
  behaviour comes only from the supplied finite future, with anti-degeneration
  witnessed separately below.
-/
def orderActionTrace {State : Type u} {Event : Type v}
    {Order : Type w} {Action : Type z}
    (step : State -> Event -> State)
    (order : State -> Event -> Order)
    (action : State -> Event -> Action) :
    State -> List Event -> OrderActionFiniteTrace Order Action :=
  runOutputTrace step (fun state event =>
    { order := order state event, action := action state event })

/--
  L0 finite-corpus fibre theorem for order/action trace output.  It reuses the
  generic finite-corpus machinery: equality of finite signatures is exactly
  equality on the supplied boundary/flip future corpus.  This is an
  anti-degeneration-friendly target contract only, not Rust bit-exact
  CompleteClass and not a full-state quotient.
-/
theorem order_action_finite_corpus_fiber_iff
    {State : Type u} {Event : Type v} {Order : Type w} {Action : Type z}
    (step : State -> Event -> State)
    (order : State -> Event -> Order)
    (action : State -> Event -> Action)
    (corpus : List (List Event)) (x y : State) :
    finiteCorpusClassify (orderActionTrace step order action) corpus x =
        finiteCorpusClassify (orderActionTrace step order action) corpus y <->
      CorpusBehEquiv (orderActionTrace step order action) corpus x y :=
  finite_corpus_fiber_iff (orderActionTrace step order action) corpus x y

/--
  L0 seed factorization for order/action trace output.  If the seed carries the
  projected transition plus order/action observations, then same seed implies
  same future trace for every supplied boundary/flip future.  This is the
  positive seed contract paired with the anti-degeneration witness below; it is
  not a Rust CompleteClass realization and not a full-state quotient.
-/
theorem order_action_same_seed_same_future_trace
    {State : Type u} {Event : Type v} {Seed : Type z}
    {Order : Type w} {Action : Type a}
    (step : State -> Event -> State)
    (order : State -> Event -> Order)
    (action : State -> Event -> Action)
    (project : State -> Seed)
    (stepSeed : Seed -> Event -> Seed)
    (orderSeed : Seed -> Event -> Order)
    (actionSeed : Seed -> Event -> Action)
    (hstep : ∀ state event, project (step state event) = stepSeed (project state) event)
    (horder : ∀ state event, order state event = orderSeed (project state) event)
    (haction : ∀ state event, action state event = actionSeed (project state) event)
    {left right : State}
    (hseed : project left = project right)
    (future : List Event) :
    orderActionTrace step order action left future =
      orderActionTrace step order action right future := by
  apply same_seed_same_future_output_trace step
    (fun state event =>
      ({ order := order state event, action := action state event } :
        OrderActionObservation Order Action))
    project stepSeed
    (fun seed event =>
      ({ order := orderSeed seed event, action := actionSeed seed event } :
        OrderActionObservation Order Action))
    hstep
  · intro state event
    simp [horder state event, haction state event]
  · exact hseed

/--
  L0 toy output for the anti-degeneration witness.  The boundary/flip condition
  is the single unit future; this intentionally separates observations/classes
  without claiming Rust CompleteClass bit-exactness or any full-state quotient.
-/
def toyOrderActionOutput (state : Bool) (_event : Unit) :
    OrderActionObservation Bool Bool :=
  { order := state, action := !state }

/--
  L0 anti-degeneration witness: the one-step boundary/flip condition separates
  two toy order/action observations/classes.  This is only a target-contract
  noncollapse example, not Rust CompleteClass realization and not a full-state
  quotient claim.
-/
theorem order_action_trace_noncollapse_witness :
    finiteCorpusClassify
        (orderActionTrace (fun state (_event : Unit) => state)
          (fun state (_event : Unit) => state)
          (fun state (_event : Unit) => !state))
        [[()]] true ≠
      finiteCorpusClassify
        (orderActionTrace (fun state (_event : Unit) => state)
          (fun state (_event : Unit) => state)
          (fun state (_event : Unit) => !state))
        [[()]] false := by
  intro h
  simp [finiteCorpusClassify, orderActionTrace, runOutputTrace] at h
  injection h with hobs
  cases hobs

end NewChanlun.Origin
