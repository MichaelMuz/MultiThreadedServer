use std::{sync::{mpsc, Arc, Mutex}, thread};
//here we have the threadpool 
pub struct ThreadPool{
    //when we spawn a thread we get a JoinHandle<> type returned to us
    // thus we store a vector of them, the thing in the <> of JoinHandle is the
    // return thype of the closure passed into each JoinHandle, our threads
    // wont return anything so we will make this a unit type <()>
    //we have changed the vector to store out new Worker struct because
    // threads take a job and perform it right away so we cannot get the function we
    // want with them
    //threads: Vec<thread::JoinHandle<()>>,
    workers: Vec<Worker>,
    //this is a sender that can send between threads, and we are sending Job's
    // accross threads
    //just changed it to Messages because it could be a terminate signal or
    // a job
    sender: mpsc::Sender<Message>,
}

//next we want to make the library crate the primary crate, to do that we need to
// make a new folder in our source directory called bin and then move main.rs
// into bin

//now we can import threadpool from this library crate into main.rs

//now we can implement our struct

impl ThreadPool{
    //we add documentation because our function could panic, this is good practice
    // if you make functions that could panic
    ///create a new ThreadPool
    /// 
    /// the size is the number of threads in the pool
    /// 
    /// # Panics
    /// 
    /// The `new` function will panic if size is 0 or less than 0
    pub fn new(size: usize) -> ThreadPool{
        assert!(size > 0);

        //here we create a Vec with the size 'size'
        let mut workers = Vec::with_capacity(size);

        let (sender, receiver) = mpsc::channel();

        let receiver = Arc::new(Mutex::new(receiver));
        //the spawn function will take a closure which will execute
        // immediately after a thread is created, this is not what we want, we
        // want to create threads that wait for some code to execute later on
        // to get the behavior we want, instead of storing threads directly,
        // we will store a struct called worker that has 2 fields, id and thread
        for id in 0..size{
            //if we just pass in receiver then the first time around ownership
            // would be taken, we want all workers to share ownership
            // of the receiver, also listening for Job's will require mutating the 
            // receiver so we want shared ownership and mutability, Arc for
            // thread safe shared ownership, and mutex for threadsafe mutability
            // that is what we did above
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        //create and return the ThreadPool
        ThreadPool{workers, sender}

    }
}

//here we will implement for Threadpool look at the bottom of the page
// to see where it is being modelled from
impl ThreadPool{
    //execute is a method thus takes self, the second argument is a generic that 
    // have the trait bounds FnOnce(), which is a closure that takes ownership
    // of enviromental variables and it has Send, which means it can be transfered
    // between threads, and has the static lifetime
    //we made our threadpool of workers but execute needs to use cross thread
    // channels to send one of our workers the closure to do, we will have the
    // ThreadPool hold onto the sending end and each worker struct hold onto
    // the receiving end, lets create a struct called Job that will contain
    // the closures we need to send down the channel
    pub fn execute<F>(&self, f: F) 
    where 
        F: FnOnce() + Send + 'static,
    {
        //first we wrap the closure we received in a Box smart pointer
        let job = Box::new(f);
        //then we will send our job down the channel
        // .send() will fail if all of our threads stop running but we know that
        // they wont as long as the pool exists
        self.sender.send(Message::NewJob(job)).unwrap();
    }
}

struct Worker{
    //an id so we cna keep track of the thread especially when debugging
    id: usize,
    //a thread that looks for jobs to be assigned
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker{
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Worker{
        //we will put the receiver inside of the thread we spawn
        //imagine our server got 4 requests at the exact same time, we have 4 
        // workers in our threadpool, one of the workers will acquire a lock to the
        // receiver and pick up a job that they will execute, as soon as the worker
        // starts executing the job the receiver will be unlocked, then another
        // worker will aquire the lock, look for a job and start executing the
        // second request, this happens for request 3 and 4 but when the 5th request
        // comes in, it has to wait for the first 4 to be finished, the first worker
        // to finish their job will aquire a lock to the reciever, pick up the job,
        // and begin to execute it
        let thread = thread::spawn(move || loop{
            //we call .lock() on the receiver to aquire a Mutex
            // which we call unwrap on becuase the lock method returns a result
            // then we call .recv() go get a job from the channel, which we must
            // unwrap, recv blocks so if no job is available then the thread that
            // aquired the Mutex to the receiver will wait until the will wait
            // for a job to become available
            let message = receiver
            .lock()
            .unwrap()
            .recv()
            .unwrap();
            
            match message{
                Message::NewJob(job) => {
                    println!("Worker {} got a job; executing.", id);
                    job();
                } 
                Message::Terminate => {
                    println!("Worker {} was told to terminate.", id);
                    break;
                }
            }
        });


        Worker{id, thread: Some(thread)}
    }
    
}
//lets implement the drop trait on ThreadPool so we can make the server stop
// by allowing the threads to finish but take no new jobs
impl Drop for ThreadPool{
    fn drop(&mut self){

        println!("Sending Terminate Message to all Worker");

        //here we are looping through our workers and sending each one a
        // Terminate Message, we use _ because we basically just send as many
        // messages as we do workers since they all have a reciever tied to this
        // sender
        for _ in &self.workers{
            self.sender.send(Message::Terminate).unwrap();
        }
        for worker in &mut self.workers{
            println!("Shutting down worker {}", worker.id);
            //calling the join method will make the associated thread to finish
            // however we get the following error: "cannot move out of 
            // `worker.thread` which is behind a mutable reference"
            // we get this error because worker is a mutable reference but the 
            // .join() method takes ownership of the self in takes as parameter
            //we need a way to move thread out of the worker instance, instead
            // of having the worker store a JoinHandle directly, we can have it 
            // store an Option containing the JoinHandle or the None varaint,
            // then we can use the take method on thread to move the JoinHandle 
            // out of the Option and replace it with the None variant
            //worker.thread.join().unwrap();
            //instead of the above line we use the if let syntax to match on
            // the sum variant, since thread.take() returns an option,
            // if it is the sum variant then thread will take the value of 
            // whats inside the Some(), if .take() returns None, we know the thread
            // is already cleaned up so we skip it
            if let Some(thread) = worker.thread.take(){
                //if the thread is still going we call .join() to wait for
                // it to finish before we continue
                //now we have to make sure the threads do not loop indefinately
                // waiting for new tasks, we make a new enum called message
                thread.join().unwrap();
                //we have added the fixes above and we still join to make sure
                // each worker has time to receive and process all Terminate
                // Messages
            }
        }
        
    }
}
enum Message{
    NewJob(Job),
    Terminate,
}
//struct Job;
//we will change our Job struct to a type alias

//now Job is a type alias for a trait object that holds the type of closure 
// ThreadPool's .execute() expects, we use a trait object here to taht all types 
// of Jobs can be passed in to .execute()
type Job = Box<dyn FnOnce() + Send + 'static>;

//we want to implement our execute method with a similar signiture to thread::spawn()
//lets look at spawn's signiture:

/*
#[stable(feature = "rust1", since "1.0.0")]
//spawn takes an argument called f which is a generic type with a few trait bounds
// such as: FnOnce which is a closure trait bound that takes ownership of the
// values in its enviroment, f also has the Send trait bound which means we can
// transfer the closure from one thread to another, and it has the static lifetime
// which means the receive can hold onto this type for as long as they need to
// and the type will be valid until the reciever drops it
pub fn spawn<F,T>(f: F) -> JoinHandle<T> 
where 
    F: FnOnce() -> T,
    F: Send + 'static,
    T: Send + 'static,   
{
    Builder::new().spawn(f).expect("failed to spawn thread");
}

*/