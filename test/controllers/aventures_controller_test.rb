require 'test_helper'

class AventuresControllerTest < ActionDispatch::IntegrationTest
  setup do
    @aventure = aventures(:one)
  end

  test "should get index" do
    get aventures_url
    assert_response :success
  end

  test "should get new" do
    get new_aventure_url
    assert_response :success
  end

  test "should create aventure" do
    assert_difference('Aventure.count') do
      post aventures_url, params: { aventure: { book_id: @aventure.book_id, charisme: @aventure.charisme, force: @aventure.force, gold: @aventure.gold, gourdes: @aventure.gourdes, gourdes_remplies: @aventure.gourdes_remplies, hp: @aventure.hp, pages_id: @aventure.pages_id, rations: @aventure.rations } }
    end

    assert_redirected_to aventure_url(Aventure.last)
  end

  test "should show aventure" do
    get aventure_url(@aventure)
    assert_response :success
  end

  test "should get edit" do
    get edit_aventure_url(@aventure)
    assert_response :success
  end

  test "should update aventure" do
    patch aventure_url(@aventure), params: { aventure: { book_id: @aventure.book_id, charisme: @aventure.charisme, force: @aventure.force, gold: @aventure.gold, gourdes: @aventure.gourdes, gourdes_remplies: @aventure.gourdes_remplies, hp: @aventure.hp, pages_id: @aventure.pages_id, rations: @aventure.rations } }
    assert_redirected_to aventure_url(@aventure)
  end

  test "should destroy aventure" do
    assert_difference('Aventure.count', -1) do
      delete aventure_url(@aventure)
    end

    assert_redirected_to aventures_url
  end
end
