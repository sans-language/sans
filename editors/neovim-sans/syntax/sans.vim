" Vim syntax file
" Language: Sans
" Maintainer: Sans Language Team

if exists("b:current_syntax")
  finish
endif

" --- Keywords ---

" Control flow
syntax keyword sansKeyword if else while for in match return spawn break continue defer select

" Declarations
syntax keyword sansDeclaration fn let mut struct enum trait impl import pub g

" Booleans
syntax keyword sansBoolean true false

" Other keywords
syntax keyword sansOther self Self channel mutex array as dyn

" --- Types ---

" Primitive types
syntax keyword sansType I F B S Int Float Bool String

" Built-in types
syntax keyword sansBuiltinType Array Option Result JsonValue HttpResponse HttpServer HttpRequest
syntax keyword sansBuiltinType Sender Receiver Mutex JoinHandle R O M HS HR Fn Map J

" --- Built-in functions ---

" I/O
syntax keyword sansBuiltinFn print p file_read fread fr file_write fwrite fw
syntax keyword sansBuiltinFn file_append fappend fa file_exists fexists fe
syntax keyword sansBuiltinFn read_file write_file args stdin_read_line srl stdin_read_bytes srb

" Conversion
syntax keyword sansBuiltinFn int_to_string str itos string_to_int stoi
syntax keyword sansBuiltinFn int_to_float itof float_to_int ftoi
syntax keyword sansBuiltinFn float_to_string ftos string_to_float stof

" Math
syntax keyword sansBuiltinFn abs min max range
syntax keyword sansBuiltinFn floor ceil round sqrt sin cos tan
syntax keyword sansBuiltinFn asin acos atan atan2 log log10 exp
syntax keyword sansBuiltinFn pow fabs fmin fmax PI E_CONST

" System
syntax keyword sansBuiltinFn sleep time now random rand print_err

" JSON
syntax keyword sansBuiltinFn json_parse jparse jp json_object jobj jo
syntax keyword sansBuiltinFn json_array jarr ja json_string jstr js
syntax keyword sansBuiltinFn json_int ji json_bool jb json_null jn
syntax keyword sansBuiltinFn json_stringify jstringify jfy

" HTTP
syntax keyword sansBuiltinFn http_get hget hg http_post hpost hp
syntax keyword sansBuiltinFn http_listen listen hl https_listen hl_s
syntax keyword sansBuiltinFn serve serve_tls stream_write stream_end
syntax keyword sansBuiltinFn cors cors_all ca ssl_ctx ssl_accept ssl_read ssl_write ssl_close
syntax keyword sansBuiltinFn ws_send ws_recv ws_close is_ws_upgrade upgrade_ws
syntax keyword sansBuiltinFn serve_file url_decode ud path_segment ps
syntax keyword sansBuiltinFn respond_json rj respond_stream query path_only
syntax keyword sansBuiltinFn content_length cl set_header
syntax keyword sansBuiltinFn set_max_workers set_read_timeout set_keepalive_timeout
syntax keyword sansBuiltinFn set_drain_timeout set_max_body set_max_headers
syntax keyword sansBuiltinFn set_max_header_count set_max_url

" Logging
syntax keyword sansBuiltinFn log_debug ld log_info li log_warn lw log_error le
syntax keyword sansBuiltinFn log_set_level ll get_log_level set_log_level

" Result / Option
syntax keyword sansBuiltinFn ok err some none

" Assert
syntax keyword sansBuiltinFn assert assert_eq assert_ne assert_ok assert_err assert_some assert_none

" Memory
syntax keyword sansBuiltinFn alloc dealloc ralloc mcpy mcmp mzero slen
syntax keyword sansBuiltinFn load8 store8 load16 store16 load32 store32 load64 store64
syntax keyword sansBuiltinFn strstr bswap16 bxor band bor bshl bshr
syntax keyword sansBuiltinFn exit system sys wfd
syntax keyword sansBuiltinFn arena_begin arena_alloc arena_end ab aa ae
syntax keyword sansBuiltinFn mget mset mhas
syntax keyword sansBuiltinFn signal_handler signal_check sigh sigc
syntax keyword sansBuiltinFn spoll gzip_compress gz setjmp longjmp
syntax keyword sansBuiltinFn panic_enable panic_disable panic_is_active panic_get_buf panic_fire
syntax keyword sansBuiltinFn pmutex_init pmutex_lock pmutex_unlock

" Pointers
syntax keyword sansBuiltinFn fptr fcall fcall2 fcall3 ptr char_at

" Path
syntax keyword sansBuiltinFn path_join pjoin path_dir pdir path_base pbase
syntax keyword sansBuiltinFn path_ext pext path_stem pstem path_is_abs pabs

" Encoding
syntax keyword sansBuiltinFn base64_encode b64e base64_decode b64d
syntax keyword sansBuiltinFn url_encode urle urld hex_encode hexe hex_decode hexd

" File system
syntax keyword sansBuiltinFn getenv genv mkdir rmdir remove rm listdir ls is_dir sh shell

" Sockets
syntax keyword sansBuiltinFn sock sbind slisten saccept srecv ssend sclose rbind rsetsockopt

" Curl
syntax keyword sansBuiltinFn cinit csets cseti cperf cclean cinfo curl_slist_append curl_slist_free

" --- Strings ---

" Triple-quoted strings (must come before regular strings)
syntax region sansTripleString start='"""' end='"""' contains=sansInterpolation
syntax region sansString start='"' skip='\\"' end='"' contains=sansEscape,sansInterpolation

syntax match sansEscape contained /\\[ntr\\"{}]/
syntax match sansInterpolation contained /\{[a-zA-Z_][a-zA-Z0-9_]*\}/

" --- Comments ---
syntax match sansComment /\/\/.*/

" --- Numbers ---
syntax match sansFloat /\<[0-9]\+\.[0-9]\+\>/
syntax match sansNumber /\<[0-9]\+\>/

" --- Operators ---
syntax match sansOperator /:=/
syntax match sansOperator /+=/
syntax match sansOperator /-=/
syntax match sansOperator /\*=/
syntax match sansOperator /\/=/
syntax match sansOperator /%=/
syntax match sansOperator /==/
syntax match sansOperator /!=/
syntax match sansOperator /<=/
syntax match sansOperator />=/
syntax match sansOperator /&&/
syntax match sansOperator /||/
syntax match sansOperator /=>/
syntax match sansOperator /::/

" --- Highlight links ---

highlight default link sansKeyword       Keyword
highlight default link sansDeclaration   Keyword
highlight default link sansBoolean       Boolean
highlight default link sansOther         Keyword
highlight default link sansType          Type
highlight default link sansBuiltinType   Type
highlight default link sansBuiltinFn     Function
highlight default link sansString        String
highlight default link sansTripleString  String
highlight default link sansEscape        SpecialChar
highlight default link sansInterpolation Special
highlight default link sansComment       Comment
highlight default link sansNumber        Number
highlight default link sansFloat         Float
highlight default link sansOperator      Operator

let b:current_syntax = "sans"
