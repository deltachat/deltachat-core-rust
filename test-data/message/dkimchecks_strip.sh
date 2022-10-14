#!/bin/bash
# This is a small script I used to strip all the unnecessary information from the realworldemails before committing them, to avoid blowing up the repo size.
# Also, I deleted deltachattest@outlook.com/deltachat-dev@posteo.de.
# Also, I anonymized them using
#   for n in ...; do rename $n alice *; done
#   for n in ...; do rename $n alice */*; done
#   for n in ...; do find ./ -type f -exec sed -i -e "s/${n}/alice/g" {} \; ;done
# (replace ... with the list of localparts in the email addresses)
set -euxo pipefail
cd dkimchecks-2022-09-28
parent_dir=$PWD
for d in *; do
	cd $d
	for file in *; do
		if ! [[ -s $file ]]; then
			rm $file || true
		else
			python3 $parent_dir/../dkimchecks_strip.py < $file > ${file}-new
			mv -f ${file}-new $file
		fi
	done
	cd $parent_dir
done
