#[derive(Debug, Clone)]
pub struct Module {
    pub globals: Vec<(String, i64)>,
    pub functions: Vec<IrFunction>,
}

#[derive(Debug, Clone)]
pub struct IrFunction {
    pub name: String,
    pub params: Vec<String>, // param register names like "arg0", "arg1"
    pub param_struct_sizes: Vec<usize>, // 0 = scalar, >0 = pointer with N fields
    pub body: Vec<Instruction>,
}

pub type Reg = String;

#[derive(Debug, Clone)]
pub enum Instruction {
    Const { dest: Reg, value: i64 },
    FloatConst { dest: Reg, value: f64 },
    BoolConst { dest: Reg, value: bool },
    StringConst { dest: Reg, value: String },
    PrintInt { value: Reg },
    PrintFloat { value: Reg },
    PrintString { value: Reg },
    PrintBool { value: Reg },
    BinOp { dest: Reg, op: IrBinOp, left: Reg, right: Reg },
    FloatBinOp { dest: Reg, op: IrBinOp, left: Reg, right: Reg },
    FloatCmpOp { dest: Reg, op: IrCmpOp, left: Reg, right: Reg },
    CmpOp { dest: Reg, op: IrCmpOp, left: Reg, right: Reg },
    StringCmpOp { dest: Reg, op: IrCmpOp, left: Reg, right: Reg },
    Not { dest: Reg, src: Reg },
    Neg { dest: Reg, src: Reg },
    Copy { dest: Reg, src: Reg },
    Call { dest: Reg, function: String, args: Vec<Reg> },
    Ret { value: Reg },
    // Control flow
    Label { name: String },
    Branch { cond: Reg, then_label: String, else_label: String },
    Jump { target: String },
    Phi { dest: Reg, a_val: Reg, a_label: String, b_val: Reg, b_label: String },
    // Global variable operations
    GlobalLoad { dest: Reg, name: String },
    GlobalStore { name: String, value: Reg },
    // Memory operations for mutable variables
    Alloca { dest: Reg },
    Store { ptr: Reg, value: Reg },
    Load { dest: Reg, ptr: Reg },
    // Struct operations
    StructAlloc { dest: Reg, num_fields: usize },
    FieldStore { ptr: Reg, field_index: usize, value: Reg },
    FieldLoad { dest: Reg, ptr: Reg, field_index: usize, num_fields: usize },
    // Enum operations
    EnumAlloc { dest: Reg, tag: i64, num_data_fields: usize },
    EnumTag { dest: Reg, ptr: Reg },
    EnumData { dest: Reg, ptr: Reg, field_index: usize },
    // Thread operations
    ThreadSpawn {
        dest: Reg,
        function: String,
        args: Vec<Reg>,
    },
    ThreadJoin {
        handle: Reg,
    },
    // Channel operations
    ChannelCreate {
        tx_dest: Reg,
        rx_dest: Reg,
    },
    ChannelSend {
        tx: Reg,
        value: Reg,
    },
    ChannelRecv {
        dest: Reg,
        rx: Reg,
    },
    // Mutex operations
    MutexCreate {
        dest: Reg,
        value: Reg,
    },
    MutexLock {
        dest: Reg,
        mutex: Reg,
    },
    MutexUnlock {
        mutex: Reg,
        value: Reg,
    },
    // Bounded channel creation
    ChannelCreateBounded {
        tx_dest: Reg,
        rx_dest: Reg,
        capacity: Reg,
    },
    // Array operations
    ArrayCreate {
        dest: Reg,
    },
    ArrayPush {
        array: Reg,
        value: Reg,
    },
    ArrayGet {
        dest: Reg,
        array: Reg,
        index: Reg,
    },
    ArraySet {
        array: Reg,
        index: Reg,
        value: Reg,
    },
    ArrayLen {
        dest: Reg,
        array: Reg,
    },
    // String operations
    StringLen {
        dest: Reg,
        string: Reg,
    },
    StringConcat {
        dest: Reg,
        left: Reg,
        right: Reg,
    },
    StringSubstring {
        dest: Reg,
        string: Reg,
        start: Reg,
        end: Reg,
    },
    IntToString {
        dest: Reg,
        value: Reg,
    },
    StringToInt {
        dest: Reg,
        string: Reg,
    },
    // File I/O
    FileRead {
        dest: Reg,
        path: Reg,
    },
    FileWrite {
        dest: Reg,
        path: Reg,
        content: Reg,
    },
    FileAppend {
        dest: Reg,
        path: Reg,
        content: Reg,
    },
    FileExists {
        dest: Reg,
        path: Reg,
    },
    // JSON constructors
    JsonParse { dest: Reg, source: Reg },
    JsonObject { dest: Reg },
    JsonArray { dest: Reg },
    JsonString { dest: Reg, value: Reg },
    JsonInt { dest: Reg, value: Reg },
    JsonBool { dest: Reg, value: Reg },
    JsonNull { dest: Reg },
    // JSON accessors
    JsonGet { dest: Reg, object: Reg, key: Reg },
    JsonGetIndex { dest: Reg, array: Reg, index: Reg },
    JsonGetString { dest: Reg, value: Reg },
    JsonGetInt { dest: Reg, value: Reg },
    JsonGetBool { dest: Reg, value: Reg },
    JsonLen { dest: Reg, value: Reg },
    JsonTypeOf { dest: Reg, value: Reg },
    // JSON mutators (no dest — use Const for expression result)
    JsonSet { object: Reg, key: Reg, value: Reg },
    JsonPush { array: Reg, value: Reg },
    // JSON serialization
    JsonStringify { dest: Reg, value: Reg },
    // HTTP Server
    HttpListen { dest: Reg, port: Reg },
    HttpAccept { dest: Reg, server: Reg },
    HttpRequestPath { dest: Reg, request: Reg },
    HttpRequestMethod { dest: Reg, request: Reg },
    HttpRequestBody { dest: Reg, request: Reg },
    HttpRespond { dest: Reg, request: Reg, status: Reg, body: Reg },
    HttpRespondWithContentType { dest: Reg, request: Reg, status: Reg, body: Reg, content_type: Reg },
    // Function references
    FnRef { dest: Reg, name: String },
    FptrNamed { dest: Reg, func_name: String },
    Fcall { dest: Reg, fn_ptr: Reg, arg: Reg },
    // Array higher-order methods
    ArrayMap { dest: Reg, array: Reg, fn_ptr: Reg },
    ArrayFilter { dest: Reg, array: Reg, fn_ptr: Reg },
    ArrayAny { dest: Reg, array: Reg, fn_ptr: Reg },
    ArrayFind { dest: Reg, array: Reg, fn_ptr: Reg },
    ArrayEnumerate { dest: Reg, array: Reg },
    ArrayZip { dest: Reg, array: Reg, other: Reg },
    // String extension methods
    StringTrim { dest: Reg, string: Reg },
    StringStartsWith { dest: Reg, string: Reg, prefix: Reg },
    StringEndsWith { dest: Reg, string: Reg, suffix: Reg },
    StringContains { dest: Reg, string: Reg, needle: Reg },
    StringSplit { dest: Reg, string: Reg, delimiter: Reg },
    StringReplace { dest: Reg, string: Reg, old: Reg, new_str: Reg },
    // Array extension methods
    ArrayPop { dest: Reg, array: Reg },
    ArrayContains { dest: Reg, array: Reg, value: Reg },
    ArrayRemove { dest: Reg, array: Reg, index: Reg },
    // Float conversions
    IntToFloat { dest: Reg, value: Reg },
    FloatToInt { dest: Reg, value: Reg },
    FloatToString { dest: Reg, value: Reg },
    // Result operations
    ResultOk { dest: Reg, value: Reg },
    ResultErr { dest: Reg, message: Reg },
    ResultIsOk { dest: Reg, result: Reg },
    ResultIsErr { dest: Reg, result: Reg },
    ResultUnwrap { dest: Reg, result: Reg },
    ResultUnwrapOr { dest: Reg, result: Reg, default: Reg },
    ResultError { dest: Reg, result: Reg },
    // Logging
    LogDebug { dest: Reg, message: Reg },
    LogInfo { dest: Reg, message: Reg },
    LogWarn { dest: Reg, message: Reg },
    LogError { dest: Reg, message: Reg },
    LogSetLevel { dest: Reg, level: Reg },
    // Kernel functions
    PrintErr { dest: Reg, message: Reg },
    WriteFd { dest: Reg, fd: Reg, message: Reg },
    // Memory primitives
    Alloc { dest: Reg, size: Reg },
    Dealloc { dest: Reg, ptr: Reg },
    Ralloc { dest: Reg, ptr: Reg, size: Reg },
    Mcpy { dest: Reg, dst_ptr: Reg, src_ptr: Reg, len: Reg },
    Mzero { dest: Reg, ptr: Reg, len: Reg },
    Mcmp { dest: Reg, a_ptr: Reg, b_ptr: Reg, len: Reg },
    Slen { dest: Reg, ptr: Reg },
    Load8 { dest: Reg, ptr: Reg },
    Store8 { dest: Reg, ptr: Reg, val: Reg },
    Load16 { dest: Reg, ptr: Reg },
    Store16 { dest: Reg, ptr: Reg, val: Reg },
    Load32 { dest: Reg, ptr: Reg },
    Store32 { dest: Reg, ptr: Reg, val: Reg },
    Bswap16 { dest: Reg, val: Reg },
    Rbind { dest: Reg, fd: Reg, addr: Reg, len: Reg },
    Rsetsockopt { dest: Reg, fd: Reg, level: Reg, opt: Reg, val_ptr: Reg, val_len: Reg },
    Load64 { dest: Reg, ptr: Reg },
    Store64 { dest: Reg, ptr: Reg, val: Reg },
    Strstr { dest: Reg, haystack: Reg, needle: Reg },
    Exit { dest: Reg, code: Reg },
    GetLogLevel { dest: Reg },
    SetLogLevel { dest: Reg, level: Reg },
    // HTTP operations
    HttpGet { dest: Reg, url: Reg },
    HttpPost { dest: Reg, url: Reg, body: Reg, content_type: Reg },
    HttpStatus { dest: Reg, response: Reg },
    HttpBody { dest: Reg, response: Reg },
    HttpHeader { dest: Reg, response: Reg, name: Reg },
    HttpOk { dest: Reg, response: Reg },
    // Socket primitives
    Sock { dest: Reg, domain: Reg, sock_type: Reg, proto: Reg },
    Sbind { dest: Reg, fd: Reg, port: Reg },
    Slisten { dest: Reg, fd: Reg, backlog: Reg },
    Saccept { dest: Reg, fd: Reg },
    Srecv { dest: Reg, fd: Reg, buf: Reg, len: Reg },
    Ssend { dest: Reg, fd: Reg, buf: Reg, len: Reg },
    Sclose { dest: Reg, fd: Reg },
    // libcurl primitives
    Cinit { dest: Reg },
    Csets { dest: Reg, handle: Reg, opt: Reg, val: Reg },
    Cseti { dest: Reg, handle: Reg, opt: Reg, val: Reg },
    Cperf { dest: Reg, handle: Reg },
    Cclean { dest: Reg, handle: Reg },
    Cinfo { dest: Reg, handle: Reg, info: Reg, buf: Reg },
    CurlSlistAppend { dest: Reg, slist: Reg, str_ptr: Reg },
    CurlSlistFree { dest: Reg, slist: Reg },
}

#[derive(Debug, Clone, Copy)]
pub enum IrBinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

#[derive(Debug, Clone, Copy)]
pub enum IrCmpOp {
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
}
