use std::{cell::RefCell, rc::Rc};

use js_sys::{Array, Boolean, Function, JsString, Number, Object, Reflect, RegExp};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::console;

pub fn get_nested_property(
    node: &JsValue,
    keys: &Vec<&str>,
    depth: usize,
) -> Result<JsValue, JsValue> {
    if depth >= keys.len() {
        return Ok(node.clone());
    }
    match node.is_object() {
        true => {
            let next = Reflect::get(&node, &JsValue::from(keys[depth]))?;
            get_nested_property(&next, keys, depth + 1)
        }
        _ => Err(JsValue::from(format!(
            "current depth is not object, key: {}",
            keys[0.min(depth - 1)],
        ))),
    }
}

pub fn set_property(target: &Object, key: &str, value: &JsValue) -> Result<bool, JsValue> {
    Reflect::set(target, &JsString::from(key), value)
}

pub fn new_obj(keys_vals: &Vec<(&str, JsValue)>) -> Result<Object, JsValue> {
    let obj = Object::new();

    for (key, value) in keys_vals {
        set_property(&obj, key, value)?;
    }

    Ok(obj)
}

pub fn to_jss(str: &str) -> JsString {
    JsString::from(str)
}

pub fn call_js_get(this: &Object, arg: &str) -> Result<JsValue, JsValue> {
    get_js_function("get", this)?.call1(this, &JsString::from(arg))
}

pub fn get_js_function(name: &str, target: &Object) -> Result<Function, JsValue> {
    Reflect::get(&target, &JsValue::from(name))?.dyn_into()
}

pub fn is_global_console_id(id: &Object) -> Result<bool, JsValue> {
    let name = JsValue::from("console");
    let is_identifier = get_js_function("isIdentifier", id)?;
    let arg = new_obj(&vec![("name", JsValue::from("console"))])?;
    let is_ident = is_identifier.call1(id, &arg.into())?;

    let scope: Object = get_nested_property(&id, &vec!["scope"], 0)?.into();
    let get_binding = get_js_function("getBinding", &scope)?;
    let has_global = get_js_function("hasGlobal", &scope)?;
    let is_binding = get_binding.call1(&scope, &name)?;
    let has_global = has_global.call1(&scope, &name)?;
    Ok(is_ident.is_truthy() && !is_binding.is_truthy() && has_global.is_truthy())
}

pub fn is_excluded(property: &Object, exclude_array: &Array) -> Result<bool, JsValue> {
    let is_identifier = get_js_function("isIdentifier", property)?;
    let mut arg = Rc::new(RefCell::new(Object::new()));
    Ok(exclude_array
        .is_undefined()
        .then(|| Array::new())
        .expect("get exclude array fail")
        .some(&mut |name: JsValue| {
            let ref_obj = &*arg.borrow_mut();
            Reflect::set(&ref_obj, &JsValue::from("name"), &JsValue::from(name)).unwrap_or_else(
                |_e| {
                    console::log_1(&_e);
                    Boolean::from(false).into()
                },
            );
            is_identifier
                .call1(property, &ref_obj)
                .unwrap_or_else(|_e| {
                    console::log_1(&_e);
                    Boolean::from(false).into()
                })
                .is_truthy()
        }))
}

pub fn is_included_console(member_expr: &Object, exclude_array: &Array) -> Result<bool, JsValue> {
    let mem_fn_get = get_js_function("get", &member_expr)?;
    let object: Object = mem_fn_get.call1(&member_expr, &to_jss("object"))?.into();
    let property: Object = mem_fn_get.call1(&member_expr, &to_jss("property"))?.into();
    if is_excluded(&property, exclude_array)? {
        return Ok(false);
    }

    if is_global_console_id(&object)? {
        return Ok(true);
    }

    let sub_obj: Object = call_js_get(&object, "object")?.into();

    let is_identifier = get_js_function("isIdentifier", &property)?;

    let arg_1 = new_obj(&vec![("name", JsValue::from("call"))])?;
    let arg_2 = new_obj(&vec![("name", JsValue::from("apply"))])?;

    Ok(is_global_console_id(&sub_obj.into())?
        && (is_identifier.call1(&property, &arg_1)?.is_truthy()
            || is_identifier.call1(&property, &arg_2)?.is_truthy()))
}

pub fn is_included_console_bind(
    member_expr: &Object,
    exclude_array: &Array,
) -> Result<bool, JsValue> {
    let object: Object = call_js_get(&member_expr, "object")?.into();

    let is_member_expression = get_js_function("isMemberExpression", &object)?;
    if is_member_expression.call0(&object)?.is_falsy() {
        return Ok(false);
    }
    let property: Object = call_js_get(&object, "property")?.into();

    if is_excluded(&property, exclude_array)? {
        return Ok(false);
    }

    let sub_obj = call_js_get(&object, "object")?;
    let m_property = call_js_get(&member_expr, "property")?;
    let is_identifier = get_js_function("isIdentifier", &property)?;
    let arg = new_obj(&vec![("name", JsValue::from("bind"))])?;
    Ok(is_global_console_id(&sub_obj.into())?
        && is_identifier.call1(&m_property, &arg)?.is_truthy())
}

pub fn create_noop(t: &Object) -> Result<JsValue, JsValue> {
    let function_expression = get_js_function("functionExpression", t)?;
    let block_statemenet = get_js_function("blockStatement", t)?;
    function_expression.call3(
        &t,
        &JsValue::null(),
        &Array::new(),
        &block_statemenet.call1(&t, &Array::new())?,
    )
}

pub fn create_vold0(t: &Object) -> Result<JsValue, JsValue> {
    let unary_expression = get_js_function("unaryExpression", t)?;
    let numeric_literal = get_js_function("numericLiteral", t)?;
    Ok(unary_expression.call2(
        &t,
        &JsString::from("void"),
        &numeric_literal.call1(t, &Number::from(0))?,
    )?)
}

pub fn include_white_log(args: &Array, white_list: &RegExp) -> Result<bool, JsValue> {
    Ok(args
        .is_array()
        .then(move || args)
        .or(Some(&Array::new()))
        .expect("这都能错？")
        .reduce(
            &mut |prev: JsValue, curr: JsValue, _, _| {
                let value = Reflect::get(&curr, &JsValue::from("value"))
                    .unwrap_or_else(|_e| {
                        console::log_1(&_e);
                        JsValue::from("")
                    })
                    .as_string()
                    .unwrap_or_else(|| "".to_string());

                (prev.is_truthy() || white_list.test(&value)).into()
            },
            &mut Boolean::from(false).into(),
        )
        .is_truthy())
}
