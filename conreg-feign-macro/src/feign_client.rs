//! # Feign client 宏实现
//!
//! 主要用于生成HTTP请求函数，构造HTTP请求。

use proc_macro::TokenStream;
use quote::quote;
use std::collections::HashSet;
use proc_macro2::Ident;
use syn::punctuated::Punctuated;
use syn::{parse_macro_input, Error, Expr, FnArg, GenericArgument, ItemTrait, Lit, LitStr, Meta, MetaNameValue, Pat, PathArguments, ReturnType, Token, TraitItem, TraitItemFn, Type};

/// 解析 feign_client 属性参数
///
/// 从属性参数中提取 service_id、base_path 和 url。
///
/// # Arguments
/// * `args` - 来自属性的 Punctuated Meta 参数列表
///
/// # Returns
/// * `Ok((service_id, base_path, url))` - 成功解析的参数
/// * `Err(Error)` - 解析失败，返回错误详情
fn parse_feign_client_args(args: &Punctuated<Meta, Token![,]>) -> Result<(String, Option<String>, Option<String>), Error> {
    let mut service_id = None;
    let mut base_path = None;
    let mut url = None;

    for meta in args {
        match meta {
            Meta::NameValue(meta) => {
                let value = parse_string_value(&meta.value)?;
                if meta.path.is_ident("service_id") {
                    service_id = Some(value);
                } else if meta.path.is_ident("base_path") {
                    base_path = Some(value);
                } else if meta.path.is_ident("url") {
                    url = Some(value);
                }
            }
            _ => return Err(Error::new_spanned(
                meta,
                "Expected format: #[feign_client(service_id = \"...\", base_path = \"...\", url = \"...\")]",
            )),
        }
    }

    service_id.ok_or_else(|| {
        Error::new(proc_macro2::Span::call_site(), "Missing required 'service_id'")
    }).map(|sid| (sid, base_path, url))
}

/// 从表达式中解析字符串字面量
///
/// # Arguments
/// * `expr` - 要解析的表达式
///
/// # Returns
/// * `Ok(String)` - 成功解析的字符串值
/// * `Err(Error)` - 表达式不是字符串字面量
fn parse_string_value(expr: &Expr) -> Result<String, Error> {
    match expr {
        Expr::Lit(expr_lit) => {
            if let Lit::Str(lit) = &expr_lit.lit {
                Ok(lit.value())
            } else {
                Err(Error::new_spanned(expr, "Value must be a string literal"))
            }
        }
        _ => Err(Error::new_spanned(expr, "Value must be a string literal")),
    }
}

pub fn feign_client_impl(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args with Punctuated::<Meta, Token![,]>::parse_terminated);
    let trait_def = parse_macro_input!(input as ItemTrait);

    // Parse arguments with error handling
    let (service_id, base_path, url) = match parse_feign_client_args(&args) {
        Ok(result) => result,
        Err(e) => return e.to_compile_error().into(),
    };

    let trait_name = &trait_def.ident;
    let impl_struct_name = Ident::new(
        &format!("{}Impl", trait_name),
        trait_name.span(),
    );

    // 从 trait 中提取方法 - 只处理函数项
    let methods: Vec<&TraitItemFn> = trait_def
        .items
        .iter()
        .filter_map(|item| {
            if let TraitItem::Fn(method) = item {
                Some(method)
            } else {
                None
            }
        })
        .collect();

    // 生成方法实现
    let method_impls = generate_method_impls(&methods, &service_id, &base_path, &url);

    // 实现 trait
    let expanded = quote! {
        // 保留原始 trait 定义
        #trait_def

        pub struct #impl_struct_name {
            lb_client: conreg_client::lb::LoadBalanceClient,
        }

        impl #impl_struct_name {
            pub fn new(lb_client: conreg_client::lb::LoadBalanceClient) -> Self {
                Self { lb_client }
            }

            pub fn with_timeout(timeout: std::time::Duration) -> Self {
                Self {
                    lb_client: conreg_client::lb::LoadBalanceClient::new_with_connect_timeout(timeout),
                }
            }
        }

        impl Default for #impl_struct_name {
            fn default() -> Self {
                Self::new(conreg_client::lb::LoadBalanceClient::new())
            }
        }

        // 为生成的结构体实现 trait
        impl #trait_name for #impl_struct_name {
            #method_impls
        }
    };

    TokenStream::from(expanded)
}

/// 为所有 trait 方法生成方法实现
///
/// 过滤掉没有 HTTP 方法属性的方法，
/// 为有效的 HTTP 端点生成实现。
fn generate_method_impls(
    methods: &[&TraitItemFn],
    service_id: &str,
    base_path: &Option<String>,
    url: &Option<String>,
) -> proc_macro2::TokenStream {
    methods
        .iter()
        .filter_map(|method| {
            extract_http_method_and_path(method).map(|(http_method, path)| {
                generate_single_method_impl(
                    method,
                    &http_method,
                    &path,
                    service_id,
                    base_path,
                    url,
                )
            })
            // 跳过没有 HTTP 方法属性的方法
        })
        .collect::<proc_macro2::TokenStream>()
}

/// 从方法属性中提取 HTTP 方法和路径
///
/// 同时从属性或文档注释中提取 query、form 和 headers 参数。
/// 支持多种格式：
/// - 简单格式：#[get("/path")]
/// - 命名参数：#[get(path="/path", query="...", ...)]
/// - 文档注释：/// HTTP_METHOD:path (回退方案)
///
/// # Arguments
/// * `method` - 要分析的 Trait 方法
///
/// # Returns
/// * `Some((http_method, path))` - 成功提取的 HTTP 方法和路径
/// * `None` - 未找到 HTTP 方法属性
fn extract_http_method_and_path(method: &TraitItemFn) -> Option<(String, String)> {
    for attr in &method.attrs {
        if let Some(ident) = attr.path().get_ident() {
            match ident.to_string().as_str() {
                http_method @ ("get" | "post" | "put" | "delete" | "patch") => {
                    // 尝试解析属性参数
                    let args_result = attr.parse_args_with(
                        Punctuated::<Meta, Token![,]>::parse_terminated
                    );

                    match args_result {
                        // 空参数：#[get()]
                        Ok(args) if args.is_empty() => {
                            return Some((http_method.to_uppercase(), String::new()));
                        }
                        // 无括号：#[get]
                        Err(_) if attr.meta.require_list().is_err() => {
                            return Some((http_method.to_uppercase(), String::new()));
                        }
                        // 有参数，继续下面的解析
                        Ok(_) => {}
                        // 解析错误，尝试简单的字符串字面量解析
                        Err(_) => {}
                    }

                    // 尝试解析为简单字符串字面量：#[get("/path")]
                    if let Ok(lit_str) = attr.parse_args::<LitStr>() {
                        return Some((http_method.to_uppercase(), lit_str.value()));
                    }

                    // 尝试解析为命名参数：#[get(path="/path", query="...", ...)]
                    return args_result.ok().and_then(|args| {
                        let mut path_value: Option<String> = None;

                        for meta in args {
                            if let Meta::NameValue(name_value) = meta {
                                if name_value.path.is_ident("path") {
                                    if let Expr::Lit(expr_lit) = &name_value.value {
                                        if let Lit::Str(lit_str) = &expr_lit.lit {
                                            path_value = Some(lit_str.value());
                                            break;
                                        }
                                    }
                                }
                            }
                        }

                        // 如果未指定 path，返回空路径
                        Some((http_method.to_uppercase(), path_value.unwrap_or_default()))
                    });
                }
                /*// 回退方案：解析文档注释格式 "HTTP_METHOD:path"
                "doc" => {
                    if let Meta::NameValue(meta) = &attr.meta {
                        if let Expr::Lit(expr_lit) = &meta.value {
                            if let Lit::Str(lit_str) = &expr_lit.lit {
                                let doc_value = lit_str.value();
                                // 解析格式为 "HTTP_METHOD:path" 的文档注释
                                if let Some(colon_pos) = doc_value.find(':') {
                                    let http_method = doc_value[..colon_pos].to_string();
                                    let path = doc_value[colon_pos + 1..].to_string();
                                    return Some((http_method, path));
                                }
                            }
                        }
                    }
                }*/
                _ => {}
            }
        }
    }
    None
}

/// 为单个方法生成实现
///
/// 构建完整的方法实现，包括：
/// - URL 构造（包含路径和查询参数）
/// - 使用适当的 HTTP 方法和 body 构建请求
/// - 根据返回类型解析响应
///
/// # Arguments
/// * `method` - 方法签名
/// * `http_method` - HTTP 方法（GET, POST 等）
/// * `path` - 请求路径模板
/// * `service_id` - 用于负载均衡的服务 ID
/// * `base_path` - 可选的基础路径前缀
/// * `url` - 可选的直接 URL（覆盖负载均衡）
fn generate_single_method_impl(
    method: &TraitItemFn,
    http_method: &str,
    path: &str,
    service_id: &str,
    base_path: &Option<String>,
    url: &Option<String>,
) -> proc_macro2::TokenStream {
    let method_name = &method.sig.ident;
    let asyncness = &method.sig.asyncness;
    let output = &method.sig.output;

    // 通过添加 base_path（如果提供）来构建完整路径
    let full_path = if let Some(base) = base_path {
        format!("{}{}", base, path)
    } else {
        path.to_string()
    };

    // 分析方法的参数以确定如何构建请求
    let param_analysis = analyze_parameters(method, &full_path);

    // 生成 URL 构建代码
    let url_building = generate_url_building(&param_analysis, &full_path, service_id, url);

    // 根据 HTTP 方法和参数生成请求构建代码
    let request_building = generate_request_building(http_method, &param_analysis);

    // 根据返回类型生成响应解析代码
    // 支持：String（文本）、Bytes、StatusCode 和 JSON（默认）
    let parse_response = generate_response_parsing(output);

    // 提取方法签名中的参数名称和类型
    let params: Vec<_> = method
        .sig
        .inputs
        .iter()
        .filter_map(|input| {
            if let FnArg::Typed(pat_type) = input {
                Some(quote! { #pat_type })
            } else {
                None
            }
        })
        .collect();

    quote! {
        #asyncness fn #method_name(&self, #(#params),*) #output {
            use reqwest::Method;

            // 使用路径参数和查询参数构建 URL
            let url = #url_building;
            
            // 构建并发送 HTTP 请求
            let response = #request_building;
            
            // 根据期望的返回类型解析响应
            #parse_response
            
            Ok(result)
        }
    }
}

/// 参数分析结果
///
/// 包含从方法参数和属性中提取的信息：
/// - 路径参数：用于替换 URL 路径中的占位符
/// - 查询参数：作为 URL 查询字符串附加
/// - 表单参数：作为表单数据或 multipart 发送
/// - Body/JSON 参数：作为请求 body 发送
/// - Header 参数：添加到 HTTP headers
#[derive(Debug, Default)]
struct ParamAnalysis {
    path_params: Vec<(Ident, Type)>,
    query_params: Vec<(Ident, Type)>,
    form_params: Vec<(Ident, Type)>,
    body_param: Option<(Ident, Type)>,
    json_param: Option<(Ident, Type)>,
    header_params: Vec<(String, String)>,
    has_multipart_form: bool,
}

/// 从模板字符串中提取 query/form 参数，如 "name={name}&age={age}"
///
/// # Arguments
/// * `template` - 包含参数占位符的模板字符串
///
/// # Returns
/// 从模板中提取的参数名称向量
fn extract_params_from_template(template: &str) -> Vec<String> {
    let mut params = Vec::new();
    let mut chars = template.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '{' {
            let mut param_name = String::new();
            while let Some(&next_ch) = chars.peek() {
                if next_ch == '}' {
                    chars.next(); // consume '}'
                    break;
                } else {
                    param_name.push(chars.next().unwrap());
                }
            }
            if !param_name.is_empty() {
                params.push(param_name);
            }
        }
    }

    params
}

/// 解析 header 模板，如 "Authorization={token}" 或 "X-Name=xxx"
///
/// # Arguments
/// * `header` - Header 模板字符串
///
/// # Returns
/// * `Some((header_name, Some(param_name)))` - 动态参数引用
/// * `Some((header_name, None))` - 静态值
/// * `None` - 格式无效
fn parse_header_template(header: &str) -> Option<(String, Option<String>)> {
    header.find('=').map(|eq_pos| {
        let header_name = header[..eq_pos].trim().to_string();
        let value_part = header[eq_pos + 1..].trim();

        // 检查是否是参数引用，如 "{token}"
        if value_part.starts_with('{') && value_part.ends_with('}') {
            let param_name = value_part[1..value_part.len() - 1].to_string();
            (header_name, Some(param_name))
        } else {
            // 静态值
            (header_name, Some(value_part.to_string()))
        }
    })
}

/// 分析方法参数以按用途对其进行分类
///
/// 检查方法属性和参数以确定：
/// - 哪些参数放在 URL 路径中（路径参数）
/// - 哪些参数放在查询字符串中
/// - 哪些参数是表单数据或 body 内容
/// - 哪些参数应该设置为 HTTP headers
///
/// # Arguments
/// * `method` - 要分析的方法签名
/// * `path` - 完整请求路径（用于识别路径参数占位符）
///
/// # Returns
/// 包含分类参数的分析结果
fn analyze_parameters(method: &TraitItemFn, path: &str) -> ParamAnalysis {
    let mut analysis = ParamAnalysis::default();

    // 从属性或文档注释中提取 query、form、body、json 和 header 参数
    let mut query_template_params = Vec::new();
    let mut form_template_params = Vec::new();
    let mut header_templates = Vec::new();
    let mut body_template = None;
    let mut json_template = None;

    for attr in &method.attrs {
        // 解析 HTTP 方法属性：#[get(path="...", query="...", ...)]
        if let Some(ident) = attr.path().get_ident() {
            if matches!(ident.to_string().as_str(), "get" | "post" | "put" | "delete" | "patch") {
                if let Ok(args) = attr.parse_args_with(
                    Punctuated::<Meta, Token![,]>::parse_terminated
                ) {
                    for meta in args {
                        match meta {
                            Meta::NameValue(ref name_value) => {
                                extract_param_from_meta(
                                    name_value,
                                    &mut query_template_params,
                                    &mut form_template_params,
                                    &mut header_templates,
                                    &mut body_template,
                                    &mut json_template,
                                );
                            }
                            Meta::List(ref meta_list) => {
                                if meta_list.path.is_ident("headers") {
                                    if let Ok(nested) = meta_list.parse_args_with(
                                        Punctuated::<LitStr, Token![,]>::parse_terminated
                                    ) {
                                        for lit_str in nested {
                                            header_templates.push(lit_str.value());
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        // 解析文档注释（回退方案或来自宏生成的文档）
        /*if let Meta::NameValue(meta) = &attr.meta {
            if let Expr::Lit(expr_lit) = &meta.value {
                if let Lit::Str(lit_str) = &expr_lit.lit {
                    let doc_value = lit_str.value();
                    if doc_value.starts_with("HTTP_QUERY:") {
                        let template = &doc_value[11..];
                        query_template_params = extract_params_from_template(template);
                    } else if doc_value.starts_with("HTTP_FORM:") {
                        let template = &doc_value[10..];
                        form_template_params = extract_params_from_template(template);
                    } else if doc_value.starts_with("HTTP_BODY:") {
                        let template = &doc_value[9..];
                        body_template = Some(template.to_string());
                    } else if doc_value.starts_with("HTTP_JSON:") {
                        let template = &doc_value[9..];
                        json_template = Some(template.to_string());
                    } else if doc_value.starts_with("HTTP_HEADER:") {
                        let header = &doc_value[12..];
                        header_templates.push(header.to_string());
                    }
                }
            }
        }*/
    }

    // 收集所有 header 参数名称，以便将它们从其他分类中排除
    let header_param_names: HashSet<_> = header_templates
        .iter()
        .filter_map(|h| parse_header_template(h).and_then(|(_, p)| p))
        .collect();

    // 根据模板和路径对方法参数进行分类
    for input in &method.sig.inputs {
        if let FnArg::Typed(pat_type) = input {
            if let Pat::Ident(pat_ident) = &*pat_type.pat {
                let param_name = pat_ident.ident.to_string();
                let param_type = &pat_type.ty;

                // 如果是 header 参数则跳过（单独处理）
                if header_param_names.contains(&param_name) {
                    continue;
                }

                let param_tuple = (pat_ident.ident.clone(), *param_type.clone());

                // 检查这是否是 multipart form 参数（按类型）
                if is_multipart_form_type(param_type) {
                    analysis.has_multipart_form = true;
                    analysis.form_params.push(param_tuple);
                } else if path.contains(&format!("{{{}}}", param_name)) {
                    // 路径参数：替换 URL 中的 {param_name}
                    analysis.path_params.push(param_tuple);
                } else if query_template_params.contains(&param_name) {
                    // 查询参数：作为 ?name=value 附加
                    analysis.query_params.push(param_tuple);
                } else if let Some(ref body_template) = body_template && body_template.contains(&param_name) {
                    // Body 参数：原始字符串 body
                    analysis.body_param = Some(param_tuple);
                } else if let Some(ref json_template) = json_template && json_template.contains(&param_name) {
                    // JSON 参数：序列化并通过 .json() 发送
                    analysis.json_param = Some(param_tuple);
                } else if form_template_params.contains(&param_name) {
                    // Form 参数：作为表单数据发送
                    analysis.form_params.push(param_tuple);
                }
                // 注意：不匹配任何模板的参数当前被忽略
                // 如果需要，可以将其改为默认查询参数
            }
        }
    }

    // 解析 header 参数并与方法参数关联
    for header_template in &header_templates {
        if let Some((header_name, param_name)) = parse_header_template(header_template) {
            if let Some(param) = param_name {
                analysis.header_params.push((header_name, param));
            }
        }
    }

    analysis
}

/// 从 Meta name-value 对中提取参数的辅助函数
///
/// 检查 meta 属性并根据属性名称（query、form、body、json、headers）
/// 更新相应的参数集合。
fn extract_param_from_meta(
    name_value: &MetaNameValue,
    query_params: &mut Vec<String>,
    form_params: &mut Vec<String>,
    header_templates: &mut Vec<String>,
    body_template: &mut Option<String>,
    json_template: &mut Option<String>,
) {
    if let Expr::Lit(expr_lit) = &name_value.value {
        if let Lit::Str(lit_str) = &expr_lit.lit {
            let template = lit_str.value();
            if name_value.path.is_ident("query") {
                *query_params = extract_params_from_template(&template);
            } else if name_value.path.is_ident("form") {
                *form_params = extract_params_from_template(&template);
            } else if name_value.path.is_ident("body") {
                *body_template = Some(template);
            } else if name_value.path.is_ident("json") {
                *json_template = Some(template);
            } else if name_value.path.is_ident("headers") {
                header_templates.push(template);
            }
        }
    }
}

/// 检查类型是否为 reqwest::multipart::Form
///
/// # Arguments
/// * `ty` - 要检查的类型
///
/// # Returns
/// 如果类型表示 multipart form 则返回 `true`
fn is_multipart_form_type(ty: &Type) -> bool {
    match ty {
        Type::Path(type_path) => {
            // 检查 reqwest::multipart::Form 或 multipart::Form
            let path_str = type_path.path.segments.iter()
                .map(|seg| seg.ident.to_string())
                .collect::<Vec<_>>()
                .join("::");
            path_str == "reqwest :: multipart :: Form" ||
                path_str == "multipart :: Form"
        }
        _ => false,
    }
}

/// 检查类型是否为 Option<T>
///
/// # Arguments
/// * `ty` - 要检查的类型
///
/// # Returns
/// 如果类型是 Option<T> 则返回 `true`
fn is_option_type(ty: &Type) -> bool {
    matches!(ty, Type::Path(type_path) if 
        type_path.path.segments.last().map_or(false, |seg| seg.ident == "Option"))
}

/// 生成 URL 构建代码
///
/// 通过以下步骤构造完整 URL：
/// 1. 从基础 URL 开始（来自直接 URL 或负载均衡）
/// 2. 替换路径参数占位符
/// 3. 如果存在查询参数则附加
///
/// # Arguments
/// * `analysis` - 参数分析结果
/// * `full_path` - 包含 base_path 前缀的完整路径
/// * `service_id` - 用于负载均衡的服务 ID
/// * `url` - 可选的直接 URL 覆盖
fn generate_url_building(
    analysis: &ParamAnalysis,
    full_path: &str,
    service_id: &str,
    url: &Option<String>,
) -> proc_macro2::TokenStream {
    // 如果提供了直接 URL，则将其作为基础；否则使用负载均衡 URL
    let mut url_expr = if let Some(base_url) = url {
        quote! { format!("{}{}", #base_url, #full_path) }
    } else {
        // 使用负载均衡 URL
        quote! { format!("lb://{}/{}", #service_id, #full_path) }
    };

    // 替换路径参数（例如 {id} -> 实际值）
    for (param_name, _) in &analysis.path_params {
        url_expr = quote! {
            #url_expr.replace(&format!("{{{}}}", stringify!(#param_name)), &#param_name.to_string())
        };
    }

    // 如果存在查询参数则添加
    if !analysis.query_params.is_empty() {
        let query_parts: Vec<_> = analysis
            .query_params
            .iter()
            .map(|(param_name, param_type)| {
                // 检查是否为 Option 类型以便有条件地包含
                if is_option_type(param_type) {
                    quote! {
                        if let Some(val) = #param_name {
                            query_parts.push(format!("{}={}", stringify!(#param_name), val));
                        }
                    }
                } else {
                    // 非 Option 查询参数 - 始终包含
                    quote! {
                        query_parts.push(format!("{}={}", stringify!(#param_name), #param_name));
                    }
                }
            })
            .collect();

        quote! {
            {
                let mut url = #url_expr;
                let mut query_parts = Vec::new();
                #(#query_parts)*
                if !query_parts.is_empty() {
                    url = format!("{}?{}", url, query_parts.join("&"));
                }
                url
            }
        }
    } else {
        url_expr
    }
}

/// 根据 HTTP 方法和参数生成请求构建代码
///
/// 处理不同类型的参数：
/// - Body 参数：作为原始字符串 body 发送
/// - JSON 参数：序列化并通过 Content-Type: application/json 发送
/// - Form 参数：作为 application/x-www-form-urlencoded 或 multipart 发送
/// - Multipart form：作为 multipart/form-data 发送
/// - Header 参数：添加到 HTTP headers
///
/// # Arguments
/// * `http_method` - HTTP 方法（GET, POST, PUT 等）
/// * `analysis` - 参数分析结果
fn generate_request_building(
    http_method: &str,
    analysis: &ParamAnalysis,
) -> proc_macro2::TokenStream {
    // 生成基础 HTTP 方法调用
    let method_quote = match http_method {
        "GET" => quote! { self.lb_client.get(&url).await },
        "POST" => quote! { self.lb_client.post(&url).await },
        "PUT" => quote! { self.lb_client.put(&url).await },
        "DELETE" => quote! { self.lb_client.delete(&url).await },
        "PATCH" => quote! { self.lb_client.patch(&url).await },
        _ => quote! { self.lb_client.request(Method::GET, &url).await },
    };

    let header_quote = if !analysis.header_params.is_empty() {
        let header_setters: Vec<_> = analysis
            .header_params
            .iter()
            .map(|(header_name, param_name)| {
                // 尝试将 param_name 解析为变量，如果解析失败则使用字面量
                if let Ok(ident) = syn::parse_str::<Ident>(param_name) {
                    quote! {
                        .header(#header_name, #ident.to_string())
                    }
                } else {
                    quote! {
                        .header(#header_name, #param_name)
                    }
                }
            })
            .collect();
    
        quote! {
            #(#header_setters)*
        }
    } else {
        quote! {}
    };

    // 对 body attribute 使用 .body()（原始字符串 body）
    let body_quote = if let Some((body_param_name, _)) = &analysis.body_param {
        quote! {
                .body(#body_param_name.to_string())

            }
    } else {
        quote! {}
    };

    // 对 json attribute 使用 .json()（序列化的 JSON body）
    let json_quote = if let Some((json_param_name, _)) = &analysis.json_param {
        quote! {
                .json(&#json_param_name)

            }
    } else {
        quote! {}
    };

    // 对 form attribute 使用 .multipart()（application/x-www-form-urlencoded 或 multipart）
    let form_quote = if let Some((form_param_name, _)) = &analysis.form_params.first() {
        quote! {
                .multipart(#form_param_name)

        }
    } else {
        quote! {}
    };
    quote! {
        #method_quote?
        #header_quote
        #body_quote
        #json_quote
        #form_quote
        .send()
        .await
        .map_err(|e| {
            crate::FeignError::RequestError(e.to_string())
        })?
    }
}

/// 生成响应解析代码
///
/// 支持不同的响应类型：
/// - () 或空元组：不期望响应 body
/// - String：纯文本响应
/// - Bytes：原始字节响应
/// - StatusCode：仅 HTTP 状态码
/// - 其他类型：JSON 反序列化（默认）
///
/// # Arguments
/// * `output` - 方法的返回类型
fn generate_response_parsing(output: &ReturnType) -> proc_macro2::TokenStream {
    let return_type = match output {
        ReturnType::Type(_, ty) => {
            // 从 Result<T, E> 中提取内部类型
            if let Type::Path(type_path) = ty.as_ref() {
                if let Some(segment) = type_path.path.segments.last() {
                    if segment.ident == "Result" {
                        if let PathArguments::AngleBracketed(args) = &segment.arguments {
                            if let Some(arg) = args.args.first() {
                                if let GenericArgument::Type(inner_ty) = arg {
                                    inner_ty
                                } else {
                                    return generate_json_parsing();
                                }
                            } else {
                                return generate_json_parsing();
                            }
                        } else {
                            return generate_json_parsing();
                        }
                    } else {
                        // 非 Result 返回类型 - 默认 JSON 解析
                        return generate_json_parsing();
                    }
                } else {
                    return generate_json_parsing();
                }
            } else {
                return generate_json_parsing();
            }
        }
        ReturnType::Default => {
            // 未指定返回类型（返回 ()）
            return quote! { let result = (); };
        }
    };

    // 匹配实际返回类型
    match return_type {
        Type::Tuple(tuple) if tuple.elems.is_empty() => {
            // 单元类型 ()
            quote! { let result = (); }
        }
        Type::Path(type_path) => {
            if let Some(segment) = type_path.path.segments.last() {
                match segment.ident.to_string().as_str() {
                    "String" => {
                        // 纯文本响应
                        quote! {
                            let result = response.text().await.map_err(|e| {
                                crate::FeignError::DeserializationError(e.to_string())
                            })?;
                        }
                    }
                    "Bytes" => {
                        // 原始字节响应
                        quote! {
                            let result = response.bytes().await.map_err(|e| {
                                crate::FeignError::DeserializationError(e.to_string())
                            })?;
                        }
                    }
                    "StatusCode" => {
                        // 仅 HTTP 状态码
                        quote! { let result = response.status(); }
                    }
                    _ => {
                        // 默认：自定义类型的 JSON 反序列化
                        generate_json_parsing()
                    }
                }
            } else {
                generate_json_parsing()
            }
        }
        _ => generate_json_parsing(),
    }
}

/// 生成 JSON 解析代码的辅助函数
///
/// 用作默认的响应解析策略。
/// 将响应 body 从 JSON 反序列化为期望的类型。
fn generate_json_parsing() -> proc_macro2::TokenStream {
    quote! {
        let result = response.json().await.map_err(|e| {
            crate::FeignError::DeserializationError(e.to_string())
        })?;
    }
}
