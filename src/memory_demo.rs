use std::rc::Rc;
// use std::cell::RefCell;

// 演示结构体
#[derive(Debug)]
struct MyData {
    id: i32,
    #[allow(dead_code)]
    content: String,
}

// Drop trait 用于监听内存释放时机
impl Drop for MyData {
    fn drop(&mut self) {
        println!("  [内存释放] MyData(id={}) 所在的内存被释放了", self.id);
    }
}

pub fn run_demo() {
    println!("\n=== Rust 内存管理与指针演示 ===");

    // 1. 栈 (Stack) vs 堆 (Heap) - Box 智能指针
    println!("\n[1] 栈 vs 堆 (使用 Box 智能指针):");
    {
        let stack_val = 10; // 存在栈上
        let heap_val = Box::new(MyData { id: 1, content: "我是堆上的数据".to_string() }); // 存在堆上
        
        println!("  栈变量: {}", stack_val);
        println!("  堆变量 (Box): {:?}", heap_val);
        // heap_val 在这里离开作用域，自动释放内存 (调用 drop)
    } 
    println!("  (作用域结束，Box 指向的堆内存已自动回收)");

    // 2. 所有权 (Ownership) 与 移动 (Move)
    println!("\n[2] 所有权机制 (Ownership & Move):");
    {
        let s1 = Box::new(MyData { id: 2, content: "所有权演示".to_string() });
        println!("  s1 拥有数据: {:?}", s1);

        let s2 = s1; // 所有权发生“移动” (Move)，s1 失效，s2 成为新主人
        // println!("s1: {:?}", s1); // 这行代码会报错，因为 s1 已经没有所有权了
        println!("  s1 所有权移动给了 s2");
        println!("  s2 现在拥有数据: {:?}", s2);
    } // s2 离开作用域，释放 id=2 的内存

    // 3. 借用 (Borrowing) - 也就是“安全指针”
    println!("\n[3] 借用/引用 (Borrowing) - 安全的指针:");
    {
        let data = MyData { id: 3, content: "原始数据".to_string() };
        
        // 不可变引用 (类似 const 指针)
        let ref1 = &data; 
        let ref2 = &data;
        println!("  可以有多个不可变引用: {:?}, {:?}", ref1, ref2);

        // 可变引用 (类似非 const 指针)
        // let ref3 = &mut data; // 报错！因为已经有不可变引用了 (读写互斥锁机制)
    }

    // 4. 引用计数 (Rc) - 共享所有权
    println!("\n[4] 引用计数智能指针 (Rc) - 类似 C++ shared_ptr:");
    {
        let shared_data = Rc::new(MyData { id: 4, content: "共享数据".to_string() });
        println!("  创建 Rc，引用计数 = {}", Rc::strong_count(&shared_data));

        let _owner2 = Rc::clone(&shared_data); // 增加引用计数，不复制数据
        println!("  克隆后，引用计数 = {}", Rc::strong_count(&shared_data));

        {
            let _owner3 = Rc::clone(&shared_data);
            println!("  内部作用域克隆，引用计数 = {}", Rc::strong_count(&shared_data));
        } // owner3 离开，计数 -1

        println!("  离开内部作用域，引用计数 = {}", Rc::strong_count(&shared_data));
    } // 计数归零，id=4 内存释放
}
