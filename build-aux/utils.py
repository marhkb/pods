import random
import re
import string
import subprocess
import tempfile
import webbrowser
from pathlib import Path
from typing import Optional, List

BOLD = '\033[1m'
BLUE = '\033[34m'
TURQUOISE = '\033[36m'
ENDC = '\033[0m'


def print_colored(header: str, text: str) -> None:
    print(f"{BOLD}{BLUE}{header}{ENDC}: {text}")


def info(text: str) -> None:
    print_colored("INFO", text)


def input_colored(header: str, text: str) -> str:
    return input(f"{BOLD}{TURQUOISE}{header}{ENDC}: {text} ")


def c_input(text: str) -> str:
    return input_colored("CONSOLE", text)


def find_in_file(pattern: str, file_directory: Path) -> List[str]:
    with file_directory.open() as file:
        return re.findall(pattern, file.read())


def find_and_replace_in_file(pattern: str, replacement: str, file_directory: Path) -> None:
    with file_directory.open(mode='r+') as file:
        file_contents = file.read()
        new_content = re.sub(pattern, replacement, file_contents, count=1)
        file.seek(0)
        file.truncate(0)
        file.write(new_content)


def create_tmp_file() -> Path:
    tmp_file_name = ''.join(random.choice(string.ascii_letters) for _ in range(10))
    tmp_file_location = tempfile.gettempdir()
    tmp_file_dir = Path(tmp_file_location, tmp_file_name)
    subprocess.run(['touch', tmp_file_dir], check=True)
    return tmp_file_dir


def launch_web_for_uri(uri: str) -> None:
    webbrowser.open(uri)


def launch_gedit_for_file(file_dir: Path) -> None:
    subprocess.run(['gedit', file_dir], check=True)


def get_user_input_from_gedit() -> Optional[List[str]]:
    tmp_file = create_tmp_file()

    while True:
        launch_gedit_for_file(tmp_file)

        with tmp_file.open() as file:
            file_output = file.read().strip().splitlines()
            file_output = [line for line in file_output if line]

            for index, line in enumerate(file_output):
                if index == 0:
                    print(line)
                else:
                    print(f"* {line}")

        if c_input("Was that right? [y/N]") in ("y", "Y"):
            return file_output

        if c_input("Do you want to try again? [y/N]") not in ("y", "Y"):
            return None


def copy_to_clipboard(text: str) -> None:
    subprocess.run(['wl-copy', text], check=True)
