# Progress
What can already be done:
- [x] Send experiments in a job-array
- [x] Retrieve the results from a given database
What needs to be implemented:
- [ ] Detect when a job has failed, and give the option to relaunch them.
	- First filter out all the failed experiments and remake a Job structure for them. (Still per benchmark suite.)
	- Update the experiments.json file to keep track of only the failed ones. But we need somehow to keep track of the busy benchmarks from previous iterations. (Maybe append the new Jobs to the new Job list?)
- [ ] Refactor a bit of the code, to make it a bit more readable
- [ ] Update the python scripts to read from the new database to put into Panda

# Structure
The current structure of benchmarks are:
-> Job: which is one (slurm)job per benchmark suite, and it does contain several experiments
--> Experiment (Every rotation of the Sniper arguments has its own experiment, it does contain several benchmarks)
---> Benchmark (Every benchmark has its own parameters to be able to be run.)
