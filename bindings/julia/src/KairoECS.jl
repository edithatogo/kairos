module KairoECS

export version_string, self_check

const VERSION_STRING = "0.1.0"

version_string() = VERSION_STRING

function self_check()
    return Dict(
        :package => "KairoECS",
        :version => VERSION_STRING,
        :status => "ok",
    )
end

end # module
