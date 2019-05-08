# These are all static assets. So, I'm shipping it with my static server.
FROM wafflespeanut/static-server

ENV SOURCE=/source
ENV ADDRESS=0.0.0.0:8000

COPY .build /source
COPY content/ /source/
RUN mv /source/styles /source/assets/styles

ENTRYPOINT ["/server"]
