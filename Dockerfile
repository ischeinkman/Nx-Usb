FROM devkitpro/devkita64
ENV PATH=$DEVKITPRO/devkitA64/bin:$PATH

#Install GCC for the CC link
RUN sudo apt-get update
RUN sudo apt-get install -y build-essential

#Install Rust
RUN curl https://sh.rustup.rs -sSf > rust-init.rs 
RUN chmod +x rust-init.rs 
RUN ./rust-init.rs -y --default-toolchain nightly
RUN rm rust-init.rs
ENV PATH=/root/.cargo/bin:$PATH
RUN rustup component add rust-src
RUN cargo install xargo

#Mount the work directory
WORKDIR workdir 
VOLUME workdir 

CMD ./makew