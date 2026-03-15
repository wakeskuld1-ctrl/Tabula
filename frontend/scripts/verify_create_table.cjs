
(async () => {
    const tableName = `test_create_${Date.now()}`;
    console.log(`Testing create_table with name: ${tableName}`);

    // 1. Create Table
    const createRes = await fetch('http://localhost:3000/api/create_table', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ table_name: tableName })
    });
    const createData = await createRes.json();
    console.log('Create Response:', createData);

    if (createData.status !== 'ok') {
        console.error('Failed to create table');
        process.exit(1);
    }

    // 2. Verify in List
    const listRes = await fetch('http://localhost:3000/api/tables');
    const listData = await listRes.json();
    const found = listData.tables.some(t => t.table_name === tableName);
    console.log(`Table ${tableName} found in list: ${found}`);

    if (!found) {
        console.error('Table not found in list');
        process.exit(1);
    }

    // 3. Write to it (Initialize/Update Cell)
    // Note: Newly created table is empty. We can insert data using update_cell (which expands rows)
    const updateRes = await fetch('http://localhost:3000/api/update_cell', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
            table_name: tableName,
            session_id: null,
            row_idx: 0,
            col_idx: 1,
            col_name: 'col1',
            old_value: '',
            new_value: 'Hello World'
        })
    });
    const updateData = await updateRes.json();
    console.log('Update Response:', updateData);

    if (updateData.status === 'error') {
        console.error('Failed to update table');
        process.exit(1);
    }

    console.log('Verification Successful!');
})();
