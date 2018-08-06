#!/usr/bin/env python3

"""Utility script for running Peter.

Usage:
  peter
  peter <command> [<arguments>...]
  peter -h | --help

Options:
  -h, --help  Print this message and exit.
"""

import sys

sys.path.append('/opt/py')

import basedir
import docopt
import gitdir.host.github
import os
import shlex
import socket
import subprocess
import traceback

CRASH_NOTICE = """To: fenhl@fenhl.net
From: {}@{}
Subject: Peter crashed

Peter crashed
"""

def bot_cmd(*args):
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.connect(('127.0.0.1', 18807))
        sock.sendall(' '.join(map(shlex.quote, args)).encode('utf-8'))

def notify_crash(exc=None):
    whoami = subprocess.run(['whoami'], stdout=subprocess.PIPE, check=True).stdout.decode('utf-8').strip()
    hostname = subprocess.run(['hostname', '-f'], stdout=subprocess.PIPE, check=True).stdout.decode('utf-8').strip()
    mail_text = CRASH_NOTICE.format(whoami, hostname)
    if exc is not None:
        mail_text += '\n' + traceback.format_exc()
    return subprocess.run(['ssmtp', 'fenhl@fenhl.net'], input=mail_text.encode('utf-8'), check=True)

def start_discord():
    bot_env = os.environ.copy()
    bot_env['DISCORD_TOKEN'] = basedir.data_dirs('fidera/config.json').json()['peter']['botToken']
    gitdir.host.github.GitHub().repo('twitter/twemoji').deploy()
    return subprocess.run(['rust', '-rR'], env=bot_env, cwd=str(gitdir.host.github.GitHub().repo('dasgefolge/peter-discord').branch_path()))

if __name__ == '__main__':
    arguments = docopt.docopt(__doc__)
    if arguments['<command>']:
        bot_cmd(arguments['<command>'], *arguments['<arguments>'])
    else:
        try:
            start_discord()
        except Exception as e:
            notify_crash(e)
            raise
