#!/bin/bash

#SBATCH --account=<ACCOUNT>
#SBATCH --job-name=<JOB>
#SBATCH --qos=batch_qos
#SBATCH --partition=batch
#SBATCH --cpus-per-task=<CORES>
#SBATCH --output=stdout_%A_%a.txt
#SBATCH --error=stderr_%A_%a.txt
#SBATCH --array=1-<TASKS>
# #SBATCH --tmp=15G #need to update the slurm.conf to be able to reserve some TmpDisk space

readonly sniper_mount="/mnt/sniper"
readonly benchmarks_mount="/mnt/benchmarks"
readonly input_mount="/mnt/input"
readonly traces_mount="/mnt/traces"

original_image=<VM_NAME>
job_suffix=${SLURM_ARRAY_JOB_ID}_${SLURM_ARRAY_TASK_ID}

check_image_exists() {
	vbox_list=`docker image ls`
	while [ $? -ne 0 ]; do
		vbox_list=`docker image ls`
	done
	var=`echo ${vbox_list} | grep -c "${original_image}"`
	if [[ ${var} -eq "0" ]] ; then
		echo "Docker image does not exist!" 1>&2
		wrap_up 0
	fi
}

wrap_up() {
	echo "Copying results."
	results_dir_exists=$1
	if [ $results_dir_exists -eq 1 ]; then
		mv ~/stdout_${job_suffix}.txt .
		mv ~/stderr_${job_suffix}.txt .

		tar czf results_${job_suffix}.tar.gz *
		
	else
		tar czf results_${job_suffix}.tar.gz ~/stdout_${job_suffix}.txt ~/stderr_${job_suffix}.txt
	fi

	scp results_${job_suffix}.tar.gz slurmslave@bacchus:results/.
	scp_output=$?

	if [ $scp_output -eq 0 ]; then
		if [ $results_dir_exists -eq 1 ]; then
	  		rm -r ${working_dir}
		else
			rm ~/results_${job_suffix}.tar.gz ~/stdout_${job_suffix}.txt ~/stderr_${job_suffix}.txt
		fi
	else
	  	echo "scp to control host failed!" 1>&2
	fi

	exit
}

copy_with_retry_from_bacchus() {
	submitted_file=$1
	rename_to=$2
	while true; do
		scp bacchus:$submitted_file $rename_to > /dev/null 2>&1
		if [ $? -eq 0 ]; then
			ssh bacchus "rm $submitted_file"
			break
		else
			sleep 1
		fi
	done
}

# ARGS: git-dir-path branch-name
checkout_git_repo() {
	cd ~/$1/master
	lockdir="gitlock"
	trap 'rm -rf "$lockdir"' SIGTERM #if job happens to be terminated by slurm
	while ! mkdir $lockdir 2>/dev/null; do
		echo "Cannot enter critical section now, waiting..."
		sleep 1m
	done
	while [ -f .git/index.lock ]; do #Should not be able to wait here...
		sleep 1m
	done

	git pull
	git checkout $2
	# check if branch-name argument was a git id or an actual branch
	DETACHED=`git branch | grep -c detached`
	GIT_ID=`git rev-parse HEAD`
	declare -g ${1^^}_GIT_ID=$GIT_ID
	echo "Branch git-id is $GIT_ID."
	if [ ! -d ../$GIT_ID ]; then
		# make a directory with the git id as name and copy the branch source code there
		cp -r ../master ../$GIT_ID
		echo `date +"%d-%m-%y"` > ../$GIT_ID/.last_used.txt
	else
		echo `date +"%d-%m-%y"` > ../$GIT_ID/.last_used.txt
	fi

	if [ $2 != "master" ]; then
		git checkout master
		if [ $DETACHED -eq 0 ]; then
			git branch -D $2
		fi
	fi

	rm -r "$lockdir"
	trap - SIGTERM #finished here. (not ideal, could still have some issues, but good enough?)
	cd ~
}

HOSTNAME=`hostname`
echo "Running on machine ${HOSTNAME}."

check_image_exists

# checkout (multiple) git repositories if necessary
# eg.: checkout_git_repo ~/sniper/master my-branch
<GIT-REPOSITORIES>

# setup a temporary directory
job_dir="running/job_${job_suffix}"
echo "Creating temporary directory ${job_dir}."
mkdir -p ${job_dir}
if [ $? -ne 0 ]; then
	echo "results job dir failed" 1>&2
	wrap_up 0
fi
cd ${job_dir}
if [ $? -ne 0 ]; then
	echo "could not cd into results dir" 1>&2
	wrap_up 0
fi
# get absolute path
working_dir=`pwd`

# copy the job and execute script from the command node, implement while sleep to make sure they are already copied
echo "Copying execution files."
cp $0 job.sh
copy_with_retry_from_bacchus ~/jobs/submitted/execute_${job_suffix}.sh execute.sh
chmod +x execute.sh

container_name="${original_image}_${job_suffix}"
echo "Starting container."
#To allow sniper to pull all the submodules, copy files etc. Mount /mnt/perflab as read only
docker run --name ${container_name} \
	--rm \
	--user root \
	-v ${working_dir}:/mnt/run \
	-w /mnt/run \
	-v /mnt/perflab:/mnt/perflab:ro \
	<DOCKER_MOUNTS>--cpus=<CORES> \
	--memory=<MEMORY>m \
	${original_image} /mnt/run/execute.sh > "${working_dir}/stdout_vm.txt" 2> "${working_dir}/stderr_vm.txt"

# archive the results and copy them to the control node
wrap_up 1
