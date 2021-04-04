fn main() {
    futures_executor::block_on(async move {
        let future = async move {
            4
        };
        futures_util::pin_mut!(future);

        allochronic_util::select![
            test: future
        ]
    })
}
