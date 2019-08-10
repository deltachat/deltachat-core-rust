# perform CI jobs on PRs and after merges to master.
# triggered from .circleci/config.yml

set -e -x

export BRANCH=${CIRCLE_BRANCH:-test7}
    
# run doxygen on c-source (needed by later doc-generation steps).
# XXX modifies the host filesystem docs/xml and docs/html directories
# XXX which you can then only remove with sudo as they belong to root

# XXX we don't do doxygen doc generation with Rust anymore, needs to be
# substituted with rust-docs 
#if [ -n "$DOCS" ] ; then 
#    docker run --rm -it -v $PWD:/mnt -w /mnt/docs deltachat/doxygen doxygen
#fi

# run everything else inside docker (TESTS, DOCS, WHEELS) 
docker run -e DCC_PY_LIVECONFIG -e BRANCH -e TESTS -e DOCS \
           --rm -it -v $(pwd):/mnt -w /mnt \
           deltachat/coredeps ci_scripts/run_all.sh

