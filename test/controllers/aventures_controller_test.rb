require 'test_helper'

class AventuresControllerTest < ActionDispatch::IntegrationTest
  setup do
    @adventure = create( :adventure )
  end

  test "should get index" do
    get adventures_url
    assert_response :success
  end

  test "should get new" do
    get new_adventure_url
    assert_response :success
  end

  test "should create adventure" do
    assert_difference('Adventure.count') do
      post adventures_url, params: {adventure: {book_id: @adventure.book_id, charisme: @adventure.charisme, force: @adventure.force, gold: @adventure.gold, gourdes: @adventure.gourdes, gourdes_remplies: @adventure.gourdes_remplies, hp: @adventure.hp, pages_id: @adventure.page_id, rations: @adventure.rations } }
    end

    assert_redirected_to adventure_url(Adventure.last)
  end

  test "should show adventure" do
    get adventure_url(@adventure)
    assert_response :success
  end

  test "should get edit" do
    get edit_adventure_url(@adventure)
    assert_response :success
  end

  test "should update adventure" do
    patch adventure_url(@adventure), params: {adventure: {book_id: @adventure.book_id, charisme: @adventure.charisme, force: @adventure.force, gold: @adventure.gold, gourdes: @adventure.gourdes, gourdes_remplies: @adventure.gourdes_remplies, hp: @adventure.hp, pages_id: @adventure.page_id, rations: @adventure.rations } }
    assert_redirected_to adventure_url(@adventure)
  end

  test "should destroy adventure" do
    assert_difference('Adventure.count', -1) do
      delete adventure_url(@adventure)
    end

    assert_redirected_to adventures_url
  end
end
