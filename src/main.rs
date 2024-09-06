use glommio::{executor, Latency, LocalExecutorBuilder, Placement, Shares};
use pyo3::prelude::*;
use std::env;
use std::fs::{self, File};
use std::process::Command;

const TARGET_DOWNLOAD_PATH: &str = "/tmp/lambda";
const TARGET_ZIP_FILE_PATH: &str = "/tmp/lambda/function.zip";
const TARGET_FUNCTION_ROOT_PATH: &str = "/tmp/lambda/python_function";

// fn prep_function(s3_uri: &str) {
//     fs::create_dir_all(TARGET_DOWNLOAD_PATH).unwrap();
//     fs::remove_dir_all(TARGET_FUNCTION_ROOT_PATH).ok();
//     fs::remove_dir_all("/tmp/sls-py-req").ok();
//
//     // AWS sdk only uses tokio, which is annoying to work with.
//     // Therefore, we make this async code sync for convenience.
//     tokio::runtime::Handle::current().block_on(async {
//         // Download and unzip
//         let config = aws_config::load_from_env().await;
//         let client = aws_sdk_s3::Client::new(&config);
//         let bucket_and_key: Vec<&str> = s3_uri.splitn(2, '/').collect();
//         let bucket = bucket_and_key[0];
//         let key = bucket_and_key[1];
//         let resp = client
//             .get_object()
//             .bucket(bucket)
//             .key(key)
//             .send()
//             .await
//             .expect("Failed to download S3 object");
//
//         let body = resp.body.collect().await.expect("Failed to read S3 object");
//         tokio::fs::write(TARGET_ZIP_FILE_PATH, body.into_bytes())
//             .await
//             .unwrap();
//     });
//
//     let file = File::open(TARGET_ZIP_FILE_PATH).unwrap();
//     let mut archive = zip::ZipArchive::new(file).unwrap();
//     archive.extract(TARGET_FUNCTION_ROOT_PATH).unwrap();
// }
//
fn main() {
    // let function_zip_s3_uri = env::var("FUNCTION_ZIP_S3_URI").unwrap();

    // prep_function(&function_zip_s3_uri);

    // Get the number of physical cpus for a thread-per-core concurrency model
    let cpus = glommio::CpuSet::online()
        .expect("Err: please file an issue with glommio")
        .filter(|l| l.numa_node == 0)
        .filter(|l| l.package % 2 == 0);

    let handle = LocalExecutorBuilder::new(Placement::Fenced(cpus))
        .spawn(|| async move {
            let num_processes = env::var("NUM_PROCESSES").unwrap().parse::<u32>().unwrap();
            // create a task queue for any given thread
            // allows work to be scheduled when you have more tasks than threads
            let tq =
                executor().create_task_queue(Shares::default(), Latency::NotImportant, "tasks");

            // collect tasks to run
            let mut tasks = Vec::new();
            for _ in 0..num_processes {
                let task = glommio::spawn_local_into(
                    async move {
                        let function_entrypoint = env::var("FUNCTION_ENTRYPOINT").unwrap();
                        let parts: Vec<&str> = function_entrypoint.split('.').collect();
                        let module_name = parts[0];
                        let entrypoint_name = parts[1];

                        pyo3::prepare_freethreaded_python();
                        Python::with_gil(|py| {
                            let sys = py.import_bound("sys").unwrap();
                            let path = sys.getattr("path").unwrap();
                            path.call_method1("append", ("myvenv\\Lib\\site-packages",))
                                .unwrap();

                            let module = PyModule::import_bound(py, module_name).unwrap();
                            let entrypoint = module.getattr(entrypoint_name).unwrap();
                            let args = ("dummy", "dummy");
                            entrypoint.call1(args).unwrap();
                        });
                    },
                    tq,
                )
                .unwrap();
                tasks.push(task)
            }

            // execute all tasks in the queue
            for task in tasks.into_iter() {
                task.await;
            }
        })
        .unwrap();

    // execute all task queues in all threads
    handle.join().unwrap();
}
