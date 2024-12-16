using JSON
using ModelingToolkit
using SymbolicUtils

using CellMLToolkit

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

function save_ml(filename, ml::CellModel; trim = false)    
    stringify = trim ? trim_full : trim_partial

    d = Dict()
    
    sys = ml.sys
    d["iv"] = var_dict(sys.iv, 0.0, stringify)
    d["params"] = unique([var_dict(first(v), last(v), stringify) for v in list_params(ml)])
    d["states"] = [var_dict(first(v), last(v), stringify) for v in list_states(ml)]
    d["algs"] = [equation(eq, stringify) for eq in get_alg_eqs(sys)]
    d["odes"] = [equation(eq, stringify) for eq in get_diff_eqs(sys)]
    d["obs"] = [equation(eq, stringify) for eq in observed(sys)]
    
    io = open(filename, "w") 
    JSON.print(io, d, 4)   
    close(io)
end
