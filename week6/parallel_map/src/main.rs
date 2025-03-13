use crossbeam_channel;
use std::{thread, time};

fn parallel_map<T, U, F>(mut input_vec: Vec<T>, num_threads: usize, f: F) -> Vec<U>
where
    F: FnOnce(T) -> U + Send + Copy + 'static,
    T: Send + 'static,
    U: Send + 'static + Default,
{
    let len = input_vec.len();
    let mut output_vec: Vec<U> = Vec::with_capacity(input_vec.len()); // 初始化输出向量
    for i in 0..len {
        output_vec.push(U::default());
    }

    // 创建通道：发送任务和接收结果
    let (task_sender, task_receiver) = crossbeam_channel::unbounded::<(usize, T)>();
    let (result_sender, result_receiver) = crossbeam_channel::unbounded::<(usize, U)>();

    // 启动工作线程
    for _ in 0..num_threads {
        let task_receiver = task_receiver.clone();
        let result_sender = result_sender.clone();
        thread::spawn(move || {
            while let Ok((index, input)) = task_receiver.recv() {
                let output = f(input); // 执行 f
                result_sender.send((index, output)).unwrap();
            }
        });
    }

    // 分发任务
    for (i, input) in input_vec.into_iter().enumerate() {
        task_sender.send((i, input)).unwrap();
    }
    drop(task_sender); // 关闭任务发送端，确保线程知道没有更多任务

    // 收集结果
    for _ in 0..len {
        let (index, output) = result_receiver.recv().unwrap();
        output_vec[index] = output;
    }

    output_vec
}

fn main() {
    let v = vec![6, 7, 8, 9, 10, 1, 2, 3, 4, 5, 12, 18, 11, 5, 20];
    let squares = parallel_map(v, 10, |num| {
        println!("{} squared is {}", num, num * num);
        thread::sleep(time::Duration::from_millis(500));
        num * num
    });
    println!("squares: {:?}", squares);
}
