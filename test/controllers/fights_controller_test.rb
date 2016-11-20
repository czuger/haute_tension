require 'test_helper'

class FightsControllerTest < ActionDispatch::IntegrationTest
  setup do
    @fight_monster = create( :fight_monster )
    @adventure = @fight_monster.adventure
  end

  test "should get show" do
    get fight_url( @adventure )
    assert_response :success
  end

  test "should patch add_monster" do
    patch fight_add_monster_url( fight_id: @adventure.id, monster_id: @fight_monster.monster_id )
    assert_redirected_to fight_url( @adventure )
  end

  test "should patch remove_monster" do
    patch fight_remove_monster_url( fight_id: @adventure.id, fight_monster_id: @fight_monster.id )
    assert_redirected_to fight_url( @adventure )
  end

  test "should patch fight_monster" do
    patch fight_fight_monster_url( fight_id: @adventure.id, fight_monster_id: @fight_monster.id )
    assert_redirected_to fight_url( @adventure )
  end

end
