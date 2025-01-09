##  Benchmarking Code (benchmark.jl)

```julia
using DifferentialEquations
using BenchmarkTools
using Sundials

include("compiler.jl")

ml = CellModel("../models/ohara_rudy_cipa_v1_2017.cellml.xml")
p = get_p(ml)
u0 = get_u0(ml)

function trial(prob)    
    return solve(prob, CVODE_BDF(), dtmax=0.1)
end

prob = ODEProblem(ml.sys, u0, (0, 5000.0), p)
S = trial(prob)
n = length(S)

println("LLVM")
b_llvm = @benchmark trial(prob) setup=(prob=ODEProblem(ml.sys, S[rand(1:n)], (0, 5000.0), p))

println("naive native")
b_native = @benchmark trial(prob) setup=(prob=ODEProblem(compile(ml.sys, "native"), S[rand(1:n)], (0, 5000.0), p))

println("wasm")
b_wasm = @benchmark trial(prob) setup=(prob=ODEProblem(compile(ml.sys, "wasm"), S[rand(1:n)], (0, 5000.0), p))

println("bytecode")
b_bytecode = @benchmark trial(prob) setup=(prob=ODEProblem(compile(ml.sys, "bytecode"), S[rand(1:n)], (0, 5000.0), p))
```

## Trials:

```julia
julia> include("benchmark.jl")

...

julia> b_llvm
BenchmarkTools.Trial: 7 samples with 1 evaluation.
 Range (min … max):  436.830 ms … 564.189 ms  ┊ GC (min … max): 0.00% … 17.73%
 Time  (median):     501.427 ms               ┊ GC (median):    8.29%
 Time  (mean ± σ):   493.752 ms ±  41.042 ms  ┊ GC (mean ± σ):  7.47% ±  6.12%

  █         █         █         █ █  █                        █  
  █▁▁▁▁▁▁▁▁▁█▁▁▁▁▁▁▁▁▁█▁▁▁▁▁▁▁▁▁█▁█▁▁█▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁█ ▁
  437 ms           Histogram: frequency by time          564 ms <

 Memory estimate: 87.31 MiB, allocs estimate: 911136.

julia> b_native
BenchmarkTools.Trial: 9 samples with 1 evaluation.
 Range (min … max):  530.931 ms … 747.479 ms  ┊ GC (min … max): 0.00% … 16.62%
 Time  (median):     570.149 ms               ┊ GC (median):    0.00%
 Time  (mean ± σ):   593.075 ms ±  70.398 ms  ┊ GC (mean ± σ):  3.83% ±  6.40%

  ▁    ▁▁  ▁ █▁                           ▁                   ▁  
  █▁▁▁▁██▁▁█▁██▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁█▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁█ ▁
  531 ms           Histogram: frequency by time          747 ms <

 Memory estimate: 90.08 MiB, allocs estimate: 1097417.

julia> b_wasm
BenchmarkTools.Trial: 9 samples with 1 evaluation.
 Range (min … max):  524.159 ms … 704.000 ms  ┊ GC (min … max): 0.00% … 16.92%
 Time  (median):     547.575 ms               ┊ GC (median):    0.00%
 Time  (mean ± σ):   580.225 ms ±  67.693 ms  ┊ GC (mean ± σ):  3.80% ±  6.41%

  ▁  ▁ ▁ █     ▁ ▁                                        ▁   ▁  
  █▁▁█▁█▁█▁▁▁▁▁█▁█▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁█▁▁▁█ ▁
  524 ms           Histogram: frequency by time          704 ms <

 Memory estimate: 90.18 MiB, allocs estimate: 1097334.

julia> b_bytecode
BenchmarkTools.Trial: 3 samples with 1 evaluation.
 Range (min … max):  1.777 s …   1.940 s  ┊ GC (min … max): 0.00% … 4.25%
 Time  (median):     1.939 s              ┊ GC (median):    0.00%
 Time  (mean ± σ):   1.885 s ± 93.626 ms  ┊ GC (mean ± σ):  1.46% ± 2.45%

  ▁                                                       █  
  █▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁█ ▁
  1.78 s         Histogram: frequency by time        1.94 s <

 Memory estimate: 90.09 MiB, allocs estimate: 1097159.
```
