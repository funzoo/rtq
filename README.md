RTQD (RusT job Queue Daemon)
===

# Description

This project contains two binary

- rtqd: the daemon which runs the jobs from job queue
- rtqa: add the job into the job queue

The job queue is stored in sqlite database, which will be created in $HOME/.rtq/rtq.db

The log directory of **rtqd** locates at $HOME/tmp/rtq_work_dir/rtq/

The working directory for individual task locates at $HOME/tmp/rtq_work_dir/tasks/

# Example

all example need to keep rtqd up.

- rtqa echo "hello world"
- rtqa bash -l -c "sleep 30 & echo oh my god 2&>1 > my_redirected_output.log"
