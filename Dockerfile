FROM wafflespeanut/rust-wasm-builder:nightly as rust
COPY . /home/rust/src
WORKDIR /home/rust/src
RUN wasm-pack build

FROM node as node
COPY --from=rust /home/rust/src /home/node/app
WORKDIR /home/node/app/pkg
RUN npm link
WORKDIR /home/node/app
RUN npm link rusty-sketch && npm install && npm run build

# These are all static assets. So, I'm shipping it with my static server.
FROM wafflespeanut/static-server

ENV SOURCE=/source
ENV ADDRESS=0.0.0.0:8000

COPY --from=node /home/node/app/.build /source
COPY content/ /source/
RUN mv /source/styles /source/assets/styles

ENTRYPOINT ["/server"]
