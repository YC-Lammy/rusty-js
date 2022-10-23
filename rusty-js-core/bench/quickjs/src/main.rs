use quick_js::{Context, JsValue};

fn main() {
    let context = Context::new().unwrap();
    let t = std::time::Instant::now();

    context.add_callback("add", || { 
        println!("hello world!");
        JsValue::Undefined
     }).unwrap();
    context.eval(r#"
    function i(){
        let a = hello;
        if ((typeof a) == "bject"){
            
        } else{
            return ()=>{a()}
        }
    }
    let y = i();
    y();
    for (i=0;i<9;i++){
        hello();
    }
    a = {p:0, o:"io"};
    hello(a, 0, 9, ...[])"#);

    println!("{}", t.elapsed().as_secs_f64()*1000.0);
}
