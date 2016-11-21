require 'test_helper'

class InventoryControllerTest < ActionDispatch::IntegrationTest

  setup do
    @adventure = create( :adventure )
  end

  test "should get show" do
    get inventory_url( @adventure )
    assert_response :success
  end

  test "should get new" do
    get new_inventory_url( id: @adventure )
    assert_response :success
  end

  test "should create inventory" do
    new_item = create( :item )
    assert_difference('@adventure.items.reload.count') do
      post inventories_url, params: { id: @adventure.id, selected_items: [ @adventure.items.first.id, new_item.id ] }
    end

    assert_redirected_to inventory_url( @adventure )
  end

  test "should destroy inventory" do
    assert_difference('@adventure.items.reload.count', -1) do
      delete inventory_url( @adventure, item_id: @adventure.items.first )
    end

    assert_redirected_to inventory_url( @adventure )
  end

end
