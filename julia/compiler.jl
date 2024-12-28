using JSON
using ModelingToolkit
using SymbolicUtils
using CellMLToolkit

const libpath::String = "../target/debug/libcell.so"

mutable struct Compiler
    ref::Ptr{Cvoid}
    mem::Vector{Float64}
    regs::Vector{Any}
    first_state::Int
    count_states::Int
    first_param::Int
    count_params::Int
end

function compile(sys)
    model = JSON.json(dictify(sys))
    ref = ccall((:compile, libpath), Ptr{Cvoid}, (Cstring,), model)
    status = unsafe_string(
        ccall((:check_status, libpath), Ptr{Cchar}, (Ptr{Cvoid},), ref)
    )
    
    if status != "Success"
        error("compilation error: $status")
    end
    
    regs = JSON.parse(
        unsafe_string(
            ccall((:define_regs, libpath), Ptr{Cchar}, (Ptr{Cvoid},), ref)
        )
    )
    
    t = map(x -> x[1]["t"], regs)
    first_state = findfirst(t .== "State") 
    count_states = sum(t .== "State") 
    first_param = findfirst(t .== "Param") 
    count_params = sum(t .== "Param") 
    
    mem = map(x -> x[2] == nothing ? 0.0 : x[2], regs)
    
    q = return Compiler(
        ref, 
        mem,
        regs,
        first_state,
        count_states,
        first_param,
        count_params,
    )
    
    finalizer(q) do x
        ccall((:finalize, libpath), Cvoid, (Ptr{Cvoid},), x.ref)
    end
    
    return q
end

function (q::Compiler)(du, u, p, t)
    q.mem[4] = t
    q.mem[q.first_state:q.first_state+q.count_states-1] .= u
    q.mem[q.first_param:q.first_param+q.count_params-1] .= p
    ccall((:run, libpath), Cvoid, (Ptr{Cvoid},Ptr{Float64},UInt), q.ref, q.mem, length(q.mem))
    du .= q.mem[q.first_state+q.count_states:q.first_state+2*q.count_states-1]
end

get_p(ml::CellModel) = [last(v) for v in list_params(ml)]
get_u0(ml::CellModel) = [last(v) for v in list_states(ml)]

#********************* Dictification *************************#

function trim_full(s) 
    s = string(s)
    s = last(split(s, '₊'))
    s = first(split(s, '('))
    return s
end

function trim_partial(s) 
    s = string(s)
    s = first(split(s, '('))
    return s
end

const cellml_ops::Dict{String, String} = Dict(
    "+" =>      "plus",
    "-" =>      "minus",
    "*" =>      "times",
    "/" =>      "divide",
    "%" =>      "rem",
    "^" =>      "power",
    "sqrt" =>   "root",
    "==" =>     "eq",
    "!=" =>     "neq",
    ">" =>      "gt",
    ">=" =>     "geq",
    "<" =>      "lt",
    "<=" =>     "leq",
    "&" =>      "and",
    "|" =>      "or",
    "⊻" =>      "xor",
    "asin" =>   "arcsin",
    "acos" =>   "arccos",
    "atan" =>   "arctan",
    "acsc" =>   "arccsc",
    "asec" =>   "arcsec",
    "acot" =>   "arccot",
    "asinh" =>  "arcsinh",
    "acosh" =>  "arccosh",
    "atanh" =>  "arctanh",
    "acsch" =>  "arccsch",
    "asech" =>  "arcsech",
    "acoth" =>  "arccoth",
    "log" =>    "ln",    
    "log10" =>  "log",
    "ceil" =>   "ceiling",    
)

function opify(op) 
    if haskey(cellml_ops, op)
        return cellml_ops[op]
    else
        return op
    end
end

var_dict(var, val, stringify) = Dict("name" => stringify(var), "val" => val)

function expr(n, stringify)
    if istree(n)
        op = operation(n)
        if op isa SymbolicUtils.BasicSymbolic
            d = expr(op, stringify)
        else
            d = Dict(
                "type" => "Tree", 
                "op" => opify(stringify(operation(n))),
                "args" => [expr(c, stringify) for c in arguments(n)] 
            )
        end
    elseif n isa SymbolicUtils.BasicSymbolic
        d = Dict(
            "type" => "Var",
            "name" => stringify(n)
        )
    elseif n isa Number
        d = Dict(
            "type" => "Const",
            "val" => Float64(n)
        )
    else
        error("unrecongnized node: $n")
    end
    
    return d
end

function equation(eq, stringify) 
    return Dict(
        "lhs" => expr(eq.lhs, stringify),
        "rhs" => expr(eq.rhs, stringify),
    )        
end

function dictify(ml::CellModel; trim = false)
    stringify = trim ? trim_full : trim_partial

    d = Dict()
    
    sys = ml.sys
    d["iv"] = var_dict(sys.iv, 0.0, stringify)
    d["params"] = unique([var_dict(first(v), last(v), stringify) for v in list_params(ml)])
    d["states"] = [var_dict(first(v), last(v), stringify) for v in list_states(ml)]
    d["algs"] = [equation(eq, stringify) for eq in get_alg_eqs(sys)]
    d["odes"] = [equation(eq, stringify) for eq in get_diff_eqs(sys)]
    d["obs"] = [equation(eq, stringify) for eq in observed(sys)]
    
    return d
end

function dictify(sys::ODESystem; trim = false)
    stringify = trim ? trim_full : trim_partial

    d = Dict()

    d["iv"] = var_dict(sys.iv, 0.0, stringify)
    d["params"] = unique([var_dict(v, 0.0, stringify) for v in parameters(sys)])
    d["states"] = [var_dict(v, 0.0, stringify) for v in unknowns(sys)]
    d["algs"] = [equation(eq, stringify) for eq in get_alg_eqs(sys)]
    d["odes"] = [equation(eq, stringify) for eq in get_diff_eqs(sys)]
    d["obs"] = [equation(eq, stringify) for eq in observed(sys)]
    
    return d
end


function save_ml(filename, ml::CellModel; trim = false)    
    d = dictify(ml; trim)    
    io = open(filename, "w") 
    JSON.print(io, d, 4)   
    close(io)
end

#************************************************************

using DifferentialEquations

function test()
    ml = CellModel("/home/shahriar/af/Julia/CellMLToolkit.jl/models/beeler_reuter_1977.cellml.xml")
    f = compile(ml.sys)
    u0 = get_u0(ml)
    p = get_p(ml)
    tspan = (0, 5000.0)
    prob = ODEProblem(f, u0, tspan, p)
    sol = solve(prob, dtmax=0.1)
    return sol
end

