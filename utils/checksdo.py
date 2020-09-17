from bs4 import BeautifulSoup
from time import sleep
from datetime import datetime, timedelta

import requests
import subprocess
import glob
import os

year = ""
month = ""
day = ""
spectrum = ""

target_wavelengths = ["94", "171", "193", "211", "304", "335"]

url = "https://helioviewer.org/jp2/AIA/"  #EX: https://helioviewer.org/jp2/AIA/   ---   2018/09/24/94/
ext = 'jp2'

alist = []
lena = [0, 0, 0, 0, 0, 0]
lenb = [0, 0, 0, 0, 0, 0]



def buildURL():
	time = datetime.now()
	time = str(time).split(" ")[0].split("-")
	year = time[0]
	month = time[1]
	day = time[2]
	urlout = url + str(year) + "/" + str(month) + "/" + str(day) + "/" 
	return(urlout)

def listFD(url, ext=''):
	page = requests.get(url).text
	soup = BeautifulSoup(page, 'html.parser')
	return [url + '/' + node.get('href') for node in soup.find_all('a') if node.get('href').endswith(ext)]

def check_SDO(URL):
	time = datetime.now()
	time = str(time).split(" ")[0].split("-")
	year = time[0]
	month = time[1]
	day = time[2]
	urlout = URL + str(year) + "/" + str(month) + "/" + str(day) + "/" 

	for wlen in target_wavelengths:
		for file in glob.glob(str(wlen) + "/*.jp2"):
			file_mod_time = datetime.fromtimestamp(os.stat(file).st_mtime)
			if(str(datetime.now() - file_mod_time).find("day") != -1): #if a file is more than 24 hours old
				print("PRUNING: " + str(file))
				os.remove(file)

	for wlen in target_wavelengths:

		url = urlout + str(wlen) + "/"
		windex = target_wavelengths.index(wlen)
		print("CHECKING: " + str(url))

		for file in listFD(url, ext):
			check = str(wlen) + "/" + str(file).split("//")[2]
			print("CHECK: " + check)
			if(os.path.isfile(str(wlen) + "/" + str(file).split("//")[2]) == False): #Don't download files you already have
				subprocess.call("wget -P " + str(wlen) + " " + str(file), shell = True)
		

if __name__ == '__main__':
	check_SDO(url)

