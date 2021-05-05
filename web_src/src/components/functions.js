// Helper function for asyncronous forEach
export async function asyncForEach(array, callback) {
    for (let index = 0; index < array.length; index++) {
        await callback(array[index], index, array);
    }
}

// Helper function to prevent clicks from continuing
export function stopPropogation(e) {
    // Prevent propogation
    e = e || window.event;
    e.stopPropagation();
}
