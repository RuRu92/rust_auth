fn task(num: &mut i32) {
    for i in 1..100 {
        *num = (*num + i) * i; // Dereference `num` to update the value it points to
    }
    test_box_fut(*num); // Pass the value, not the reference
}
