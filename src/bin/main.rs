use std::fs;
use std::net::TcpListener;
use std::net::TcpStream;
use std::io::prelude::*;
use std::thread;
use std::time::Duration;
//import threadpool from lib.rs
use multithreadedServer::ThreadPool;

/*
the problem with a single threaded server is that if we get 2 requests around the 
same time, the server will process the first request and only once it is finished
get to the second request and process that which could mean a long wait for the
second one. To simulate this we will update the handle_connection() function to
process a new route which will simulate a slow request
*/
fn main() {
    let listener = 
    TcpListener::bind("127.0.0.1:7878").unwrap();

    //we will be rewriting this pare below
    /*
    for stream in listener.incoming(){
        //we use shadowing to make stream just a TcpStream, we dont handle the
        // error case
        let stream = stream.unwrap();
        handle_connection(stream);
    }
    */
    //lets think about its public api so we will write out how we would want 
    // to use our thread pool then we can implement the details, this will make 
    // sure our public api makes sense and is ergonomic to callers
    
    //first lets see how the code would look if we spawned a new thread for every
    // incoming connection
    /* 
    for stream in listener.incoming(){
        let stream = stream.unwrap();
        //this is not ideal because we want to limit the amount of threads
        thread::spawn(|| {
            handle_connection(stream);
        });
        
    }
    */

    //we want our threadpool to use this notation, where we make a new threadpool
    // with the amount of threads we want passed in, in this case 4
    let pool = ThreadPool::new(4);

    
    for stream in listener.incoming(){
        let stream = stream.unwrap();
        //we want using the threadpool to be similar to spawned threads so
        // we will make a function that takes a closure for what the next
        // thread must do, this implementation makes it easy for threadpool callers
        // to use a threadpool for anything thy want, now that we know how we
        // want it to work for us we will implement our threadpool in a library 
        // crate so that it is seperate from our server and can be used in other
        // programs 
        pool.execute(|| {
            handle_connection(stream);
        });
        
    }
    
    //the loop above is good for constantly running the server
    //here we are going to use our termination we implemented to do 2 tasks
    // and terminate afterward
    //listener.incoming() returns an iterator of the incoming requests
    // we call the .take() method which creates a new iterator yeilding
    // the first x elements, 2 in this case
    /*
    for stream in listener.incoming().take(2){
        let stream = stream.unwrap();

        pool.execute(|| {
            handle_connection(stream)
        });

    }
    */
    //now when the main function ends, the ThreadPool goes out of scope and 
    // is dropped thus the drop() function is called because of the Drop trait
    // being implemented, all of the workers get Terminate signals and everything
    // slowly stops
}

fn handle_connection(mut stream: TcpStream){

    let mut buffer = [0; 1024];

    stream.read(&mut buffer).unwrap();

    let get = b"GET / HTTP/1.1\r\n";

    //this would be the first line of a GET request to the sleep route
    // which is if someone put the site and /sleep after it
    let sleep = b"GET /sleep HTTP/1.1\r\n";

    let (status_line, filename) = 
        if buffer.starts_with(get){
            ("HTTP/1.1 200 OK", "index.html")
        }
        //if it is the sleep route we return the same index.html but slowly 
        // by letting the thread sleep for 5 seconds
        else if buffer.starts_with(sleep){
            thread::sleep(Duration::from_secs(5));
            ("HTTP/1.1 200 OK", "index.html")
        }
        else{
            ("HTTP/1.1 404 NOT FOUND", "404.html")
        };
      
        
    let contents = fs::read_to_string(filename).unwrap();
    
    
    let response = format!(
        "{}\r\nContent-Length: {}\n\r\n\r{}",
        status_line,
        contents.len(),
        contents
    );

    
    stream.write(response.as_bytes()).unwrap();
  
    stream.flush().unwrap();
}
//we can now simulate being the second request by opening 2 browser tabs the first
// going to /sleep and see how the second one has to wait for the first's long
// request

//there are many ways to handle slow requests backin up the server, but the way
// we will do it is by using a thread pool, we will have a fixed number of threads
// in a thread pool, lets say 10 threads, then when requests come into a server
// a thread will pick up a request and start processing it and once it is done 
// it returns to the thread pool to take on new requests, this means our server
// will be able to handle mutltiple reqests concurrently, we have a fixed number
// of threads because we do not want someone to issue a ton of requests which 
// would spin up a bunch of threads and waste server resources, essentaill taking
// it down, so in this case we will be able to handle 10 requests concurrently