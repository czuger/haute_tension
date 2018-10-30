module BelongingsHelper

  def gold_update_link(group_elem)
    if action_name == 'add_gold'
      add_gold_update_belongings_path( group_elem )
    else
      loose_gold_update_belongings_path( group_elem )
    end
  end

end
