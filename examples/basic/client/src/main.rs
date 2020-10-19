fn main() {
    #[cfg(windows)]
    run()
}

#[cfg(windows)]
fn run() {
    use com::{
        interfaces::iclass_factory::IClassFactory,
        interfaces::iunknown::IUnknown,
        runtime::{create_instance, get_class_object, init_apartment, ApartmentType},
    };
    use interface::{Food, IAnimal, ICat, IDomesticAnimal, IExample, CLSID_CAT_CLASS};
    // Initialize the COM apartment
    init_apartment(ApartmentType::SingleThreaded)
        .unwrap_or_else(|hr| panic!("Failed to initialize COM Library{:x}", hr));
    println!("Initialized apartment");

    // Get a `BritishShortHairCat` class factory
    let factory = get_class_object::<IClassFactory>(&CLSID_CAT_CLASS)
        .unwrap_or_else(|hr| panic!("Failed to get cat class object 0x{:x}", hr));
    println!("Got cat class object");

    // Get an instance of a `BritishShortHairCat` as the `IUnknown` interface
    let unknown = factory
        .create_instance::<IUnknown>()
        .expect("Failed to get IUnknown");
    println!("Got IUnknown");

    // Now get a handle to the `IAnimal` interface
    let animal = unknown
        .query_interface::<IAnimal>()
        .expect("Failed to get IAnimal");
    println!("Got IAnimal");

    // Call some functions on the `IAnimal` interface
    let food = Food { deliciousness: 10 };
    unsafe { animal.Eat(&food) };
    assert!(unsafe { animal.Happiness() } == 10);

    // Get a handle to new interface `IDomesticAnimal` which is actually implemented
    // in a different VTable from `IAnimal`
    let domestic_animal = animal
        .query_interface::<IDomesticAnimal>()
        .expect("Failed to get IDomesticAnimal");
    println!("Got IDomesticAnimal");

    // Safely query across interface hierarchies
    // Get a handle to an `ICat` from an `IDomesticAnimal` even though they
    // belong to different interface hierarchies and have different vtables
    let new_cat = domestic_animal
        .query_interface::<ICat>()
        .expect("Failed to get ICat");
    println!("Got ICat");
    // Call a method on the interface `ICat` interface
    unsafe { new_cat.IgnoreHumans() };

    // Get another handle to a `IDomesticAnimal` and call a method on it
    let domestic_animal_two = domestic_animal
        .query_interface::<IDomesticAnimal>()
        .expect("Failed to get second IDomesticAnimal");
    println!("Got IDomesticAnimal");
    unsafe { domestic_animal_two.Train() };

    // Call a method on a parent interface
    unsafe { domestic_animal.Eat(&food) };

    // Directly cast a child interface into a parent without going through `query_interface`
    let animal: IAnimal = domestic_animal.into();
    unsafe { animal.Eat(&food) };

    // Get another instance of `BritishShortHairCat` from the factory
    let cat = create_instance::<ICat>(&CLSID_CAT_CLASS)
        .unwrap_or_else(|hr| panic!("Failed to get a cat {:x}", hr));
    println!("Got another cat");
    unsafe { cat.Eat(&food) };

    assert!(animal.query_interface::<ICat>().is_some());
    assert!(animal.query_interface::<IUnknown>().is_some());
    // Ensure that getting an interface that the class doesn't implement returns none
    assert!(animal.query_interface::<IExample>().is_none());
    assert!(animal.query_interface::<IDomesticAnimal>().is_some());
}
