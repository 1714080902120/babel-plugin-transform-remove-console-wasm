mod tool;

use std::{
    cell::{Ref, RefCell},
    rc::Rc,
};

use js_sys::{wasm_bindgen, Array, JsString, Object, Reflect, RegExp};
use tool::{
    call_js_get, create_noop, create_vold0, get_js_function, get_nested_property,
    include_white_log, is_included_console, is_included_console_bind,
};
use wasm_bindgen::prelude::*;

// #[derive(Debug)]
// struct SafeRegExp(RegExp);

// unsafe impl Sync for SafeRegExp {}
// unsafe impl Send for SafeRegExp {}

// static REG_EXP_PTR: AtomicPtr<UnsafeCell<Mutex<SafeRegExp>>> = AtomicPtr::new(ptr::null_mut());

// fn init_global_reg_exp (reg: RegExp) {
//     let mutex = Mutex::new(SafeRegExp(reg));
//     let ptr = Box::into_raw(Box::new(UnsafeCell::new(mutex)));
//     REG_EXP_PTR.store(ptr as *mut _, Ordering::SeqCst);
// }

// fn get_reg_exp_ptr() -> Option<*mut Mutex<SafeRegExp>> {
//         let ptr = REG_EXP_PTR.load(Ordering::SeqCst);
//         if !ptr.is_null() {
//             Some(ptr as *mut Mutex<SafeRegExp>)
//         } else {
//             None
//         }
// }

fn call_expression(
    mut path: Object,
    mut state: Object,
    white_list: Ref<RegExp>,
    types: Ref<Object>,
) -> Result<(), JsValue> {
    let callee: Object = call_js_get(&path, "callee")?.into();
    let is_member_expression = get_js_function("isMemberExpression", &callee)?;

    if is_member_expression.call0(&callee)?.is_falsy() {
        return Ok(());
    }

    let state_opts_exclude: Array =
        get_nested_property(&state, &vec!["opts", "exclude"], 0)?.into();
    // unsafe {
    let arguments: Array = get_nested_property(&callee, &vec!["parent", "arguments"], 0)
        .unwrap_or_else(|_e| Array::new().into())
        .into();
    // let reg_ptr = get_reg_exp_ptr().expect("get global reg exp fail");
    // let white_list = (*reg_ptr).lock().expect("get global reg exp fail");
    if is_included_console(&callee, &state_opts_exclude)? {
        let white_list = &*white_list;

        if !include_white_log(&arguments, &white_list)? {

            let parent_path = get_nested_property(&path, &vec!["parentPath"], 0)?.into();
            let is_expression_statement = get_js_function("isExpressionStatement", &parent_path)?;
            if is_expression_statement.call0(&parent_path)?.is_truthy() {
                let remove = get_js_function("remove", &path)?;
                remove.call0(&path)?;
            } else {
                let replace_with = get_js_function("replaceWith", &path)?;
                let types = &*types;

                replace_with.call1(&path, &create_vold0(&types)?)?;
            }
        }
    } else if is_included_console_bind(&callee, &state_opts_exclude)? {
        let replace_with = get_js_function("replaceWith", &path)?;
        let types = &*types;
        replace_with.call1(&path, &create_noop(&types)?)?;
    }
    // }

    Ok(())
}

fn member_expression_exit(
    mut path: Object,
    mut state: Object,
    white_list: Ref<RegExp>,
    types: Ref<Object>,
) -> Result<(), JsValue> {
    let exclude: Array = get_nested_property(&state, &vec!["opts", "exclude"], 0)?.into();
    let parent_path = get_nested_property(&path, &vec!["parentPath"], 0)?.into();
    let is_member_expression = get_js_function("isMemberExpression", &parent_path)?;

    if is_included_console(&path, &exclude)? && is_member_expression.call0(&parent_path)?.is_falsy()
    {
        let arguments: Array = get_nested_property(&path, &vec!["parent", "arguments"], 0)
            .unwrap_or_else(|_e| Array::new().into()).into();
        if !include_white_log(&arguments, &*white_list)? {
            let is_assignment_expression = get_js_function("isAssignmentExpression", &parent_path)?;
            let parent_key: JsString = get_nested_property(&path, &vec!["parentKey"], 0)?.into();
            let types = &*types;

            if is_assignment_expression.call0(&parent_path)?.is_truthy()
                && parent_key.eq(&JsString::from("left"))
            {
                let right = call_js_get(&parent_path, "right")?.into();
                let replace_with = get_js_function("replaceWith", &right)?;
                replace_with.call1(&right, &create_noop(&types)?)?;
            } else {
                let replace_with = get_js_function("replaceWith", &path)?;
                replace_with.call1(&path, &create_noop(&types)?)?;
            }
        }
    }

    Ok(())
}

fn inner_func(mut node: Object, white_list: &Vec<String>) -> Result<JsValue, JsValue> {
    let white_list = Rc::new(RefCell::new(RegExp::new(
        &white_list.join("|"),
        Default::default(),
    )));

    // init_global_reg_exp(white_list);

    let types = Rc::new(RefCell::new(
        get_nested_property(&node, &vec!["types"], 0)?.into(),
    ));

    let plugin = Object::new();
    Reflect::set(
        &plugin,
        &JsString::from("name"),
        &JsString::from("transform-remove-console"),
    )?;

    // visitor start
    let visitor = Object::new();
    let white_list_1 = white_list.clone();
    let types_1 = types.clone();
    Reflect::set(
        &visitor,
        &JsString::from("CallExpression"),
        &Closure::wrap(Box::new(move |path, state| {
            call_expression(
                path,
                state,
                white_list_1
                    .try_borrow()
                    .expect("try borrow white_list_1 fail"),
                types_1.try_borrow().expect("try borrow types_1 fail"),
            )
        })
            as Box<dyn FnMut(Object, Object) -> Result<(), JsValue> + 'static>)
        .into_js_value(),
    )?;

    // member expression start
    let member_expression = Object::new();
    let white_list_2 = white_list.clone();
    let types_2 = types.clone();

    Reflect::set(
        &member_expression,
        &JsString::from("exit"),
        &Closure::wrap(Box::new(move |path, state| {
            member_expression_exit(
                path,
                state,
                white_list_2
                    .try_borrow()
                    .expect("try borrow white_list_2 fail"),
                types_2.try_borrow().expect("try borrow types_2 fail"),
            )
        })
            as Box<dyn FnMut(Object, Object) -> Result<(), JsValue> + 'static>)
        .into_js_value(),
    )?;

    // member expression end

    Reflect::set(
        &visitor,
        &JsString::from("MemberExpression"),
        &member_expression,
    )?;
    // visitor end

    Reflect::set(&plugin, &JsString::from("visitor"), &visitor)?;

    Ok(plugin.into())
}

/// 这里得搞个高阶函数，用来存储WHITE_LIST
#[wasm_bindgen]
pub fn init(white_list: &Array) -> JsValue {
    let white_list = white_list
        .iter()
        .map(|value| value.clone().as_string().unwrap())
        .collect::<Vec<String>>();

    Closure::wrap(
        Box::new(move |mut obj: Object| inner_func(obj, &white_list))
            as Box<dyn FnMut(Object) -> Result<JsValue, JsValue>>,
    )
    .into_js_value()
}
