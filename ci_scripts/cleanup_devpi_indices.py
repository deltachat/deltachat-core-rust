"""
Remove old "dc" indices except for master which always stays.

"""
from requests import Session
import datetime
import sys
import subprocess

MAXDAYS=7

session = Session()
session.headers["Accept"] = "application/json"


def get_indexes(baseurl, username):
    response = session.get(baseurl + username)
    assert response.status_code == 200
    result = response.json()["result"]
    return result["indexes"]


def get_projectnames(baseurl, username, indexname):
    response = session.get(baseurl + username + "/" + indexname)
    assert response.status_code == 200
    result = response.json()["result"]
    return result["projects"]


def get_release_dates(baseurl, username, indexname, projectname):
    response = session.get(baseurl + username + "/" + indexname + "/" + projectname)
    assert response.status_code == 200
    result = response.json()["result"]
    dates = set()
    for value in result.values():
        if "+links" not in value:
            continue
        for link in value["+links"]:
            for log in link["log"]:
                dates.add(tuple(log["when"]))
    return dates


def run():
    baseurl = "https://m.devpi.net/"
    username = "dc"
    for indexname in get_indexes(baseurl, username):
        projectnames = get_projectnames(baseurl, username, indexname)
        if indexname == "master" or not indexname:
            continue
        assert projectnames == ["deltachat"]
        for projectname in projectnames:
            dates = get_release_dates(baseurl, username, indexname, projectname)
            if not dates:
                print(
                    "%s has no releases" % (baseurl + username + "/" + indexname),
                    file=sys.stderr)
                date = datetime.datetime.now()
            else:
                date = datetime.datetime(*max(dates))
            if (datetime.datetime.now() - date) > datetime.timedelta(days=MAXDAYS):
                assert username and indexname
                url = baseurl + username + "/" + indexname
                subprocess.check_call(["devpi", "index", "-y", "--delete", url])



if __name__ == '__main__':
    run()
