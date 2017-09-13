FROM python:2.7.13-alpine3.6

MAINTAINER Ravi Shankar <wafflespeanut@gmail.com>

WORKDIR /usr/src/app

RUN apk update
RUN apk add --no-cache build-base jpeg-dev zlib-dev

COPY requirements.txt ./
ENV LIBRARY_PATH=/lib:/usr/lib
RUN pip install --no-cache-dir -r requirements.txt

RUN apk del build-base

COPY src ./

ENV PORT 5000
ENTRYPOINT ["python", "./runner.py"]
